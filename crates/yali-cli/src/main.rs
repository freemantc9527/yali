//! yali — 鸭梨 CLI。所有入口（workflow/GUI/剪贴板）共用的内核门面。
//! M-1 占位：子命令签名先行，实现随 T-003+ 填充。

use clap::{Parser, Subcommand};

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
    List,
    Show { name: String },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Profiles { action: ProfilesAction::Show { name } } => {
            // 骨架实现：default 档输出与 yali-core 一致（T-002 换成 ProfileStore 读取）
            let p = yali_core::Profile::default_profile();
            if name != "default" {
                anyhow::bail!("profile '{name}' not found (M-1 骨架仅内置 default)");
            }
            println!("{}", serde_json::to_string_pretty(&p)?);
            Ok(())
        }
        Cmd::Profiles { action: ProfilesAction::List } => {
            println!("default");
            Ok(())
        }
        _ => {
            println!("yali v{}: 该子命令尚未实现（M-1 骨架）", yali_core::CORE_VERSION);
            Ok(())
        }
    }
}
