#[macro_use]
extern crate clap;

use clap::Parser;
use os_xtask_utils::{BinUtil, Cargo, CommandExt, Qemu};
use std::{
    fs, path::{Path, PathBuf}, process::{self, Command}, sync::OnceLock
};

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
    Asm(AsmArgs)
}

#[derive(Args, Default)]
struct BuildArgs {
    /// 选择构建平台
    #[clap(short, long)]
    target: Option<String>,
    /// Debug模式
    #[clap(short, long)]
    debug: bool,
}

impl BuildArgs {
    fn build(&self, binary: bool) -> PathBuf {
        let target = match &self.target {
            Some(string) => {
                if string == "riscv" {
                    return "riscv64gc-unknown-none-elf".into()
                }
                else {
                    panic!();
                }
            },
            None => {
                "riscv64gc-unknown-none-elf"
            },
        };
        Command::new("python").arg("./user/build.py").status().unwrap();
        Cargo::build()
            .package("los")
            .conditional(!self.debug, |cargo| {
                cargo.release();
            })
            .target(target)
            .invoke();
        let elf = project_path()
            .join("target")
            .join(target)
            .join(if self.debug {"debug"} else {"release"})
            .join("los");
        if binary {
            let bin = elf.with_extension("bin");
            BinUtil::objcopy()
                .arg(elf)
                .args(["--strip-all", "-O", "binary"])
                .arg(&bin)
                .invoke();
            bin
        }
        else {
            elf
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
        let sbi = project_path()
            .join("sbi")
            .join("rustsbi-qemu.bin");
        let los = BuildArgs::build(&self.build, true);
        let system = Qemu::system("riscv64")
            .args(["-machine", "virt"])
            .arg("-nographic")
            .args(["-bios", sbi.to_str().unwrap()])
            .arg("-device")
            .arg(format!("loader,file={kernel},addr=0x80200000", kernel = los.to_str().unwrap()))
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
    name: Option<String>
}

impl AsmArgs {
    fn dump(self) {
        let elf = self.build.build(false);
        let out = project_path().join("target").join(self.name.unwrap_or(format!(
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
        },
        Asm(args) => {
            args.dump();
        },
    }
}
