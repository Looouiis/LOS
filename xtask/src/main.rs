#[macro_use]
extern crate clap;

use clap::Parser;
use os_xtask_utils::{BinUtil, Cargo, CommandExt, Qemu};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::OnceLock,
};

mod fs_std;

fn project_path() -> &'static Path {
    static PROJECT: OnceLock<&'static Path> = OnceLock::new();
    PROJECT.get_or_init(|| Path::new(std::env!("CARGO_MANIFEST_DIR")).parent().unwrap())
}

#[derive(Parser)]
#[clap(name = "LOS")]
#[clap(version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 构建LOS
    Build(BuildArgs),
    /// 运行LOS
    Run(RunArgs),
    /// 反汇编LOS
    Asm(AsmArgs),
}

fn main() {
    let build = BuildArgs {
        target: Some("riscv".to_string()),
        debug: false,
        pack: true,
    };
    build.build(true);
}

#[derive(Args, Default)]
struct BuildArgs {
    /// 选择构建平台
    #[clap(short, long)]
    target: Option<String>,
    /// Debug模式
    #[clap(short, long)]
    debug: bool,
    // 是否打包镜像文件
    #[clap(short, long)]
    pack: bool
}

impl BuildArgs {
    fn get_target(&self) -> &str {
        match &self.target {
            Some(string) => {
                if string == "riscv" {
                    return "riscv64gc-unknown-none-elf".into();
                } else {
                    panic!();
                }
            }
            None => "riscv64gc-unknown-none-elf",
        }
    }

    fn base_path(&self) -> PathBuf {
        let target = self.get_target();
        project_path()
            .join("target")
            .join(target)
            // .join(if self.debug { "debug" } else { "release" })
            .join("release")
    }

    fn build(&self, binary: bool) -> (PathBuf, PathBuf) {
        let target = self.get_target();
        let fs_img = project_path()
            .join("target")
            .join(target)
            .join("release").join("fs.img");
        Cargo::build()
            .package("user")
            .release()
            .target(target)
            .invoke();
        // if self.pack {
            self.fs_pack(&fs_img);
        // }
        // Cargo::build()
        //     .package("los")
        //     // .conditional(!self.debug, |cargo| {
        //     // cargo.release();
        //     // })
        //     .release()
        //     .target(target)
        //     .invoke();
        let elf = project_path()
            .join("target")
            .join(target)
            // .join(if self.debug { "debug" } else { "release" })
            .join("release")
            .join("los");
        if binary {
            let bin = elf.with_extension("bin");
            BinUtil::objcopy()
                .arg(elf)
                .args(["--strip-all", "-O", "binary"])
                .arg(&bin)
                .invoke();
            (bin, fs_img)
        } else {
            (elf, fs_img)
        }
    }
}

#[derive(Args, Default)]
struct RunArgs {
    #[clap(flatten)]
    build: BuildArgs,
}

impl RunArgs {
    fn run(&self) {
        let sbi = project_path().join("sbi").join("rustsbi-qemu.bin");
        let (los, fs_img) = BuildArgs::build(&self.build, true);
        let system = Qemu::system("riscv64")
            .args(["-machine", "virt"])
            .arg("-nographic")
            .args(["-bios", sbi.to_str().unwrap()])
            .args(["-device", format!("loader,file={kernel},addr=0x80200000", kernel = los.to_str().unwrap()).as_str()])
            .args(["-drive", format!("file={fs_img},if=none,format=raw,id=x0", fs_img = fs_img.to_str().unwrap()).as_str()])
            .args(["-device", "virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0"])
            .conditional(self.build.debug, |qemu| {
                qemu.args(["-s", "-S"]);
            })
            .as_mut()
            .status();
        if let Err(e) = system {
            println!("Error: {e}");
            process::exit(1);
        }
    }
}

#[derive(Args, Default)]
struct AsmArgs {
    #[clap(flatten)]
    build: BuildArgs,
    #[clap(long, short)]
    name: Option<String>,
}

impl AsmArgs {
    fn dump(self) {
        let (elf, _) = self.build.build(false);
        let out = project_path()
            .join("target")
            .join(self.name.unwrap_or(format!(
                "{}.asm",
                elf.file_stem().unwrap().to_string_lossy()
            )));
        println!("Asm file dumps to '{}'.", out.display());
        fs::write(out, BinUtil::objdump().arg(elf).arg("-d").output().stdout).unwrap();
    }
}

// fn main() {
//     use Commands::*;
//     match Cli::parse().command {
//         Build(args) => {
//             args.build(true);
//         }
//         Run(args) => {
//             args.run();
//         }
//         Asm(args) => {
//             args.dump();
//         }
//     }
// }
