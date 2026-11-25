//! 跨平台 golden-hash 基线工具（ADR 0002 的验证入口，change `cross-platform-golden-hash`）。
//!
//! 本 example 完全离线：固定配置、固定命令脚本、固定帧数，全部输入都是编译期常量，
//! 不读文件 / 网络 / 环境变量 / 命令行参数 / 标准输入，不使用墙钟、随机数、窗口或资产。
//! 只调用 `w3-core` 的公开 API，不新增、不改写任何权威状态，也不改动内核实现。
//!
//! 输出是逐帧绝对哈希文本，供 Windows 与 Linux 两端逐字节 diff：
//! - 每帧一行 `frame=<十进制帧号> hash=0x<16 位定宽小写十六进制>`；
//! - 末尾有且仅有一行汇总 `frames=<N> final_hash=0x<016x> schema=<编译期常量>`；
//! - 所有行以 LF 结束，不含时间戳、路径、主机名或工具链探测值等平台相关信息。
//!
//! P8.5a 基线：新增英雄复活三组权威状态进入 `compute_state_hash()`、复活
//! 冻结常量段进入 `compute_defs_hash()`，逐帧哈希布局实质变化，故基线必变。
//! 当时的 Windows 侧取值为 `final_hash=0xada6506e1da36c11`、整份文本 sha256
//! `8b48b8f2035bd7dc3df4b37c8e21c9d896cdc6caa2a367c5dc23f3df04fc99a3`
//! （41 行 / 1371 字节）。
//!
//! P8.5b 基线重取（Q1 / Q7 / tasks 7.3）：新增主城升级三组权威状态进入
//! `compute_state_hash()`、主城升级冻结常量段 + 类别 / tier 需求链进入
//! `compute_defs_hash()`，`SNAPSHOT_VERSION` / `SIM_VERSION` bump 4→5，
//! 逐帧哈希布局再次实质变化，故基线必变、不沿用 P8.5a v4 值、不声称逐字节
//! 不变。本文件输出格式未变，`GOLDEN_SCHEMA` 仍为 `golden-hash/1`；记录值
//! 仅作人工比对留档，不作自动断言。
//!
//! Windows 侧（Windows 11 / x86_64 / rustc 1.99.1）按新实现重取：
//! - 41 行 / 1371 字节，`final_hash=0xce89a7baf4d711ec`；
//! - 整份文本 sha256 `202b7490f94cbc059a266c1663fc21e847e55ebb2c3620750f4346855475742c`。
//! ADR 0002 复核记录与 P8.5a 的旧值（`final_hash=0x3d7e4312f63844e5` /
//! `0xada6506e1da36c11`）已失效，不沿用、不声称逐字节不变。
//! **Debian 12 x86_64 侧 v4 / v5 离线逐字节比对仍待人工 / VM 执行；macOS、
//! 其它 CPU 架构（ARM64 等）与其它 rustc 版本仍属待验证项**，不得据本记录
//! 扩大为跨平台确定性结论或声称本 change 有新验证（Q7）。
//!
//! P8.5c 基线重取（Q4 / Q12 / tasks 7.5）：新增野怪 / 营地 / 中立归属 /
//! leash 状态机 / 掉落事实 / 地面物品 / 物品实例 / 击杀收益登记 / 确定性
//! 随机码消费序列等权威状态进入 `compute_state_hash()`，野怪冻结常量段 /
//! 营地脚本 / 掉落表分层 / 中立类别登记进入 `compute_defs_hash()`，
//! `SNAPSHOT_VERSION` / `SIM_VERSION` bump 5→6，逐帧哈希布局再次实质
//! 变化，故基线必变、不沿用 P8.5b v5 值、不声称逐字节不变。
//!
//! Windows 侧（Windows 11 / x86_64 / rustc 1.99.1）按 v6 实现重取：
//! - 41 行 / 1371 字节，`final_hash=0xc802b63cd6c55b74`；
//! - 整份文本 sha256 `c5927605d534348e4e92db6ecb4042a42abfa7cd9077fe70da5283717c4f7d7e`。
//! 上文 P8.5b 的 `final_hash=0xce89a7baf4d711ec` 已失效，不沿用。
//! **Debian 12 x86_64 侧 v4 / v5 / v6 逐字节离线比对仍待人工 / VM 执行；
//! macOS、其它 CPU 架构（ARM64 等）与其它 rustc 版本仍属待验证项**，
//! 不得据本记录扩大为跨平台确定性结论或声称本 change 有新验证（Q12）。
//!
//! P8.5d 基线重取（Q4 / Q12 / tasks 3.4）：新增顶层权威状态 `item_shop`
//! （六格定长实例背包 / 复活死亡背包 / 拾取购买使用丢弃移交售卖事实 /
//! 商店库存与刷新计数 / 回城传送施法帧目标次数 / 随机码消费序列 / 购买
//! 扣费登记）进入 `compute_state_hash()`，商店 / 道具 / 回城冻结定义段
//! 进入 `compute_defs_hash()`，`ItemInstanceState` 扩展 `holder` / `slot`，
//! `SNAPSHOT_VERSION` / `SIM_VERSION` bump 6→7，逐帧哈希布局再次实质
//! 变化，故基线必变、不沿用 P8.5c v6 值、不声称逐字节不变。
//!
//! Windows 侧（Windows 11 / x86_64 / rustc 1.99.1）按 v7 实现重取：
//! - 41 行 / 1371 字节，`final_hash=0x940cb016f6630f1e`；
//! - 整份文本 sha256 `d6c0c944d6ac3397cd16995ab82bce3243ecd7673d4788fec336c332936aaa41`。
//! 上文 P8.5c v6 的 `final_hash=0xc802b63cd6c55b74` 已失效、不沿用、
//! 不声称逐字节不变。**Debian 12 x86_64 侧 v4 / v5 / v6 / v7 逐字节
//! 离线比对仍待人工 / VM 执行；macOS、其它 CPU 架构（ARM64 等）与其它
//! rustc 版本仍属待验证项**，不得据本记录扩大为跨平台确定性结论或
//! 声称本 change 有新验证（Q12）；不声称 Debian 12 侧 v4 / v5 /
//! v6 / v7 逐字节比对已完成。
//!
//! P8 → P9 基线重取（Q2 / Q3 / Q5' / tasks 3.3 / 4.4）：新增 `w3-core`
//! 顶层权威状态 `faction_mechanics` 进入 `compute_state_hash()`，
//! 族特色机制冻结常量段 + 回血条件定义层类别登记进入
//! `compute_defs_hash()`，`SNAPSHOT_VERSION` / `SIM_VERSION` bump 7→8，
//! 逐帧哈希布局再次实质变化，故基线必变、不沿用 P8.5d v7 值、
//! 不声称逐字节不变。本文件输出格式未变，`GOLDEN_SCHEMA` 仍为
//! `golden-hash/1`；记录值仅作人工比对留档，不作自动断言。
//!
//! Windows 侧（Windows 11 / x86_64 / rustc 1.99.1）按 v8 实现重取：
//! - 41 行 / 1371 字节，`final_hash=0x0b53a047b1f8e5e0`；
//! - 整份文本 sha256 `aa0754e2640badd6fb93cbfeb7b73cdb54fcc70d4497a50ca29d2c84d9d628d4`。
//! 上文 P8.5d v7 的 `final_hash=0x940cb016f6630f1e` 已失效、不沿用、
//! 不声称逐字节不变。**Debian 12 x86_64 侧 v4 / v5 / v6 / v7 / v8
//! 逐字节离线比对仍待人工 / VM 执行；macOS、其它 CPU 架构（ARM64 等）
//! 与其它 rustc 版本仍属待验证项**，不得据本记录扩大为跨平台确定性
//! 结论或声称本 change 有新验证（Q5' / Q8'）；不声称 Debian 12 侧
//! v4 / v5 / v6 / v7 / v8 逐字节比对已完成。
//!
//! P11 基线重取（Q3 / Q4 / Q5 / Q8 / tasks 7.3）：新增 `WorldGameplayState.abilities`
//! 技能系统权威运行态（zone / 召唤 / 光环受益集 / 被动 proc / 变身 /
//! buff 分类 / 链接 / `rng_draws` / 进行中施法）进入 `compute_state_hash()`，
//! 技能清单 / 行为目录 / buff 分类 / 冻结常量段进入 `compute_defs_hash()`，
//! `SNAPSHOT_VERSION` / `SIM_VERSION` bump 8→9，逐帧哈希布局再次实质变化，
//! 故基线必变、不沿用 P9 v8 值 `0x0b53a047b1f8e5e0`、不声称逐字节不变。
//! 本文件输出格式未变，`GOLDEN_SCHEMA` 仍为 `golden-hash/1`；记录值仅作
//! 人工比对留档，不作自动断言。
//!
//! Windows 侧（Windows 11 / x86_64 / rustc 1.99.1）按 v9 实现重取：
//! - 41 行 / 1371 字节，`final_hash=0x240467029c2b423c`；
//! - 整份文本 sha256 `4bb9e9f860a060bdaf1b0785c3307de35fa8c10bd1d8e70759b1b8041350fa60`。
//! 上文 P9 的 `final_hash=0x0b53a047b1f8e5e0` 已失效、不沿用、不声称逐字节
//! 不变。**Debian 12 x86_64 侧 v4 / v5 / v6 / v7 / v8 / v9、macOS、
//! 其它 CPU 架构（ARM64 等）与其它 rustc 版本的逐字节离线比对仍待 VM / 人工
//! 执行**，不得据本记录扩大为跨平台确定性结论或声称本 change 有新验证
//! 结论（Q6 / Q8）；不声称 Debian 12 侧 v4–v9 逐字节比对已完成。

use w3_core::{
    Command, CommandKind, EntityId, Frame, MatchConfig, RtsMatch, SimFixed, StaticWorld,
};

/// 固定种子，与 `determinism.rs::config()` 一致。
const SEED: u64 = 0x5eed;
/// ADR 0002 冻结的 30 Hz 逻辑帧率。
const LOGIC_FPS: u32 = 30;
/// 固定内容哈希。
const CONTENT_HASH: u64 = 0xC0FFEE;
/// 合成静态世界的宽高（不读取地图数据）。
const WORLD_WIDTH: u32 = 16;
const WORLD_HEIGHT: u32 = 16;
/// 固定推进帧数，与 `determinism.rs` 交错测试的观测窗口一致。
const FRAME_COUNT: u64 = 40;
/// 输出格式与工具版本：编译期常量字面量，两端源码相同即字节相同。
const GOLDEN_SCHEMA: &str = "golden-hash/1";

/// 由整数字面量构造 Q47.16 定点值；显式处理错误，不静默吞错。
fn fixed(value: i64) -> SimFixed {
    SimFixed::from_int(value).expect("golden_hash: reason=fixed_conversion_failed")
}

/// 生成命令；初值全部为编译期常量。
fn spawn_kind(x: i64, y: i64) -> CommandKind {
    CommandKind::Spawn {
        x: fixed(x),
        y: fixed(y),
        hp: 100,
        speed: fixed(3),
        attack_damage: 100,
        attack_range: fixed(100),
    }
}

/// 移动命令；`group` 与 `append` 分别覆盖群体规划与追加订单。
fn move_kind(entity: EntityId, x: i64, y: i64, group: u32, append: bool) -> CommandKind {
    CommandKind::Move {
        entity,
        x: fixed(x),
        y: fixed(y),
        group,
        append,
    }
}

/// 硬编码的固定命令脚本，全部数值均为编译期字面常量。
///
/// 骨架与 `determinism.rs::script()` 同构：帧 0 玩家 1 / 2 各一个 Spawn
/// （按 `PlayerId -> Sequence -> Arrival` 稳定排序分配 ID 1 / 2）、帧 1 两个 Move、
/// 帧 6 一个 Stop、帧 12 一个 Move。此后追加 Spawn / Attack / SetObstacle /
/// RemoveObstacle / 群体移动 / 追加订单 / Noop，以覆盖更多确定性代码路径：
/// 攻击结算、动态占地与重规划、分帧群体规划、待执行订单队列、稳定排序中的空命令。
fn script() -> Vec<Command> {
    let first = EntityId::new(1).expect("golden_hash: reason=invalid_entity_id");
    let second = EntityId::new(2).expect("golden_hash: reason=invalid_entity_id");
    let third = EntityId::new(3).expect("golden_hash: reason=invalid_entity_id");

    vec![
        // 帧 0：两个 Spawn，稳定排序分配 ID 1 / 2。
        Command::new(Frame::new(0), 1, 1, spawn_kind(0, 0)),
        Command::new(Frame::new(0), 2, 1, spawn_kind(0, 0)),
        // 帧 1：两个普通移动，含负坐标（由世界边界 clamp）。
        Command::new(Frame::new(1), 1, 2, move_kind(first, 5, 3, 0, false)),
        Command::new(Frame::new(1), 2, 2, move_kind(second, -4, 2, 0, false)),
        // 帧 2：第三个单位，作为攻击目标，分配 ID 3。
        Command::new(Frame::new(2), 3, 1, spawn_kind(2, 2)),
        // 帧 3：同帧两次攻击，覆盖攻击结算与存活复核。
        Command::new(
            Frame::new(3),
            1,
            3,
            CommandKind::Attack {
                attacker: first,
                target: third,
            },
        ),
        Command::new(
            Frame::new(3),
            2,
            3,
            CommandKind::Attack {
                attacker: second,
                target: third,
            },
        ),
        // 帧 4：新增动态占地，触发在途单位确定性重规划。
        Command::new(
            Frame::new(4),
            1,
            4,
            CommandKind::SetObstacle {
                id: 1,
                x: 3,
                y: 0,
                w: 1,
                h: 4,
            },
        ),
        // 帧 6：Stop，与参考脚本一致。
        Command::new(Frame::new(6), 1, 5, CommandKind::Stop { entity: first }),
        // 帧 8：移除占地，再次触发重规划。
        Command::new(Frame::new(8), 1, 6, CommandKind::RemoveObstacle { id: 1 }),
        // 帧 10：新增第二个动态占地。
        Command::new(
            Frame::new(10),
            2,
            4,
            CommandKind::SetObstacle {
                id: 2,
                x: 6,
                y: 4,
                w: 2,
                h: 2,
            },
        ),
        // 帧 12：Move，与参考脚本一致。
        Command::new(Frame::new(12), 2, 5, move_kind(second, 9, 9, 0, false)),
        // 帧 14：同帧群体移动，覆盖分帧群体规划与稳定 ID 顺序。
        Command::new(Frame::new(14), 1, 7, move_kind(first, 7, 7, 7, false)),
        Command::new(Frame::new(14), 2, 6, move_kind(second, 7, 7, 7, false)),
        // 帧 18：追加订单，覆盖待执行订单队列。
        Command::new(Frame::new(18), 1, 8, move_kind(first, 1, 1, 0, true)),
        // 帧 20：Noop，覆盖不改变状态的命令也参与稳定排序。
        Command::new(Frame::new(20), 3, 2, CommandKind::Noop),
        // 帧 24：移除第二个动态占地。
        Command::new(Frame::new(24), 2, 7, CommandKind::RemoveObstacle { id: 2 }),
    ]
}

fn main() {
    let config = MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: CONTENT_HASH,
    };
    let world = StaticWorld::open_world(WORLD_WIDTH, WORLD_HEIGHT, SimFixed::ONE);
    let mut sim = RtsMatch::create_with_world(config, world);

    // 全部命令在任何 step() 之前一次性提交；到达序号按脚本顺序冻结。
    for command in script() {
        sim.submit_command(command)
            .expect("golden_hash: reason=command_rejected");
    }

    for _ in 0..FRAME_COUNT {
        // 帧号取步进前的逻辑帧：step() 提交后时钟 +1，与 0 基编号保持一致。
        let frame = sim.read_view().tick;
        sim.step();
        let hash = sim.compute_state_hash();
        print!("frame={frame} hash=0x{hash:016x}\n");
    }

    // 汇总行：最终哈希由同一纯函数重新计算，必然等于最后一帧的哈希。
    let final_hash = sim.compute_state_hash();
    print!("frames={FRAME_COUNT} final_hash=0x{final_hash:016x} schema={GOLDEN_SCHEMA}\n");
}
