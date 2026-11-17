//! yali — 鸭梨 CLI。所有入口（workflow/GUI/剪贴板）共用的内核门面。
//! T-002：profiles 子命令接入 ProfileStore（磁盘读取 + 出厂播种）。
//! compress/convert/lossless/undo/doctor 待 T-003+ 填充真实管道。

use clap::{Parser, Subcommand};
use yali_core::ProfileStore;

#[derive(Parser)]
#[command(name = "yali", version, about = "鸭梨 Yali — 让图片没有压力")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 压缩图片（右键「压一下」走的就是这条路径）
    Compress {
        files: Vec<String>,
        #[arg(short, long, default_value = "default")]
        profile: String,
        #[arg(long)]
        quiet: bool,
    },
    /// 格式转换
    Convert { files: Vec<String>, to: String },
    /// JXL 无损包裹归档（可逆）
    Lossless { files: Vec<String> },
    /// 按撤销清单还原
    Undo { dir: String },
    /// Profile 管理
    Profiles {
        #[command(subcommand)]
        action: ProfilesAction,
    },
    /// 环境自检（引擎路径/版本，报 issue 时先跑这个）
    Doctor,
}

#[derive(Subcommand)]
enum ProfilesAction {
    /// 列出磁盘上所有档（首启自动播种五档出厂 fixtures）
    List,
    /// 从磁盘读取指定档，输出 JSON（与 fixtures/*.json 逐字段一致）
    Show { name: String },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Profiles { action } => match action {
            ProfilesAction::List => {
                let store = ProfileStore::open()?;
                for name in store.list()? {
                    println!("{name}");
                }
                Ok(())
            }
            ProfilesAction::Show { name } => {
                let store = ProfileStore::open()?;
                let profile = store.load(&name)?;
                println!("{}", serde_json::to_string_pretty(&profile)?);
                Ok(())
            }
        },
        Cmd::Compress { .. }
        | Cmd::Convert { .. }
        | Cmd::Lossless { .. }
        | Cmd::Undo { .. }
        | Cmd::Doctor => {
            println!("yali v{}: 该子命令尚未实现（T-003 起填充管道）", yali_core::CORE_VERSION);
            Ok(())
        }
    }
}
