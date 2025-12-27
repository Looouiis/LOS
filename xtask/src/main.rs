#[macro_use]
extern crate clap;

use clap::Parser;
use os_xtask_utils::{BinUtil, Cargo, CommandExt, Qemu};
use std::{
    env, fs::{self, read_dir}, io, path::{Path, PathBuf}, process, sync::OnceLock
};

mod fs_std;

fn target_path() -> &'static Path {
    static PROJECT: OnceLock<PathBuf> = OnceLock::new();
    PROJECT.get_or_init(|| {
        let target_dir = env::var("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let manifest_dir = env!("CARGO_MANIFEST_DIR");
                PathBuf::from(manifest_dir).parent().unwrap().join("target").to_path_buf()
            });
        target_dir
    }).as_path()
}

fn project_path() -> &'static Path {
    static PROJECT: OnceLock<PathBuf> = OnceLock::new();
    PROJECT.get_or_init(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
    }).as_path()
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

#[derive(Args, Default)]
struct BuildArgs {
    /// 选择构建平台
    #[clap(short, long)]
    target: Option<String>,
    /// Debug模式
    #[clap(short, long)]
    debug: bool,
    // 是否打包镜像文件
    #[clap(long)]
    force_pack: bool
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
        target_path()
            .join(target)
            // .join(if self.debug { "debug" } else { "release" })
            .join("release")
    }

    fn need_pack(&self) -> io::Result<bool> {
        if self.force_pack {
            return Ok(true);
        }
        let fs_img = target_path()
            .join(self.get_target())
            .join("release").join("fs.img");
        let target_time = fs::metadata(fs_img)?.modified()?;
        let src = project_path().join("user").join("src").join("bin");
        for file in read_dir(&src)? {
            let path = file?.path();
            let src_time = fs::metadata(path)?.modified()?;
            if src_time > target_time {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn build(&self, binary: bool) -> (PathBuf, PathBuf) {
        let target = self.get_target();
        let fs_img = target_path()
            .join(target)
            .join("release").join("fs.img");
        Cargo::build()
            .package("user")
            .release()
            .target(target)
            .invoke();
        self.need_pack()
            .map_or_else(
                |e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        true
                    } else {
                        panic!("fetchtime failed: {:?}", e);
                    }
                },
                |val| val,
            )
            .then(|| {
                println!("start packing...");
                println!("dir = {}", fs_img.display());
                self.fs_pack(&fs_img).expect("pack failed");
            });
        Cargo::build()
            .package("los")
            // .conditional(!self.debug, |cargo| {
            // cargo.release();
            // })
            .release()
            .target(target)
            .invoke();
        let elf = target_path()
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
        let out = target_path()
            .join(self.name.unwrap_or(format!(
                "{}.asm",
                elf.file_stem().unwrap().to_string_lossy()
            )));
        println!("Asm file dumps to '{}'.", out.display());
        fs::write(out, BinUtil::objdump().arg(elf).arg("-d").output().stdout).unwrap();
    }
}

fn main() {
    use Commands::*;
    match Cli::parse().command {
        Build(args) => {
            args.build(true);
        }
        Run(args) => {
            args.run();
        }
        Asm(args) => {
            args.dump();
        }
    }
}
