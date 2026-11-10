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
