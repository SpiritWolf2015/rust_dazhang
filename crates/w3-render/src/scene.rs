//! 只读消费边界与表现层场景状态。
//!
//! `Scene` 是表现层**唯一**持有实体状态的地方：它只消费 `WorldView` 的
//! 所有权拷贝与 `Event` 切片，从不持有 `RtsMatch`，因此结构上无法反写
//! 任何权威模拟状态（见 ADR 0002 与 `3d-rendering` spec）。
//! 视觉插值、动画切换、模型缺失降级全部只改本结构内的 `f32` / 枚举数据。

use std::collections::{BTreeMap, BTreeSet};

use w3_core::{ANGLE_SCALE, Event, EventKind, SCALE, WorldView};

use crate::model::{ModelLibrary, team_color};
use crate::terrain::{TerrainMesh, TerrainVertex};

/// 定点世界坐标（Q47.16）→ 表现层 `f32` 世界单位。
pub const fn world_to_f32(raw: i64) -> f32 {
    raw as f32 / SCALE as f32
}

/// 离散角（一整圈 = `ANGLE_SCALE`）→ 弧度，仅供表现层旋转。
pub fn facing_to_radians(raw: i64) -> f32 {
    raw as f32 / ANGLE_SCALE as f32 * std::f32::consts::TAU
}

/// 一帧只读输入：调用方从 `RtsMatch::read_view()` / `drain_events()` 取得。
/// 生命周期只覆盖一次 `consume` 调用，`Scene` 不会保存它。
pub struct FrameInput<'a> {
    pub view: &'a WorldView,
    pub events: &'a [Event],
}

/// GPU 顶点；`#[repr(C)]` + `bytemuck::Pod`，可直接上传。
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    /// 地表索引；单位标记为 -1。
    pub surface: f32,
}

/// 表现层实例：`EntityId → 视觉状态`。全部为派生数据，绝不回写内核。
#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    pub id: u64,
    /// 插值后的当前视觉位置（世界单位）。
    pub x: f32,
    pub y: f32,
    /// 上一帧 / 本帧逻辑位置，用于两帧之间插值。
    pub prev_x: f32,
    pub prev_y: f32,
    pub target_x: f32,
    pub target_y: f32,
    /// 朝向（弧度）。
    pub facing: f32,
    pub team: u8,
    pub hp: i64,
    pub alive: bool,
    /// 模型资源键；`None` 表示资源缺失，单位不显示。
    pub model_key: Option<String>,
    /// 当前动画序列索引。
    pub sequence: usize,
}

impl Instance {
    /// 两帧之间视觉插值：只改 `x` / `y`，不触碰 `target_*` 或任何权威状态。
    pub fn interpolate(&mut self, alpha: f32) {
        let alpha = alpha.clamp(0.0, 1.0);
        self.x = self.prev_x + (self.target_x - self.prev_x) * alpha;
        self.y = self.prev_y + (self.target_y - self.prev_y) * alpha;
    }
}

/// 表现层持有的场景状态。`BTreeMap` 保证稳定迭代顺序，使 readback 可复现。
#[derive(Default)]
pub struct Scene {
    instances: BTreeMap<u64, Instance>,
    /// 已消费事件的最高 sequence，用于重复投递去重。
    max_sequence: Option<u64>,
    /// 最近消费的逻辑帧号。
    last_tick: u64,
    /// 实际应用的事件数（诊断用，表现层元数据）。
    applied_events: u64,
}

fn switch_sequence(instance: &mut Instance, library: &ModelLibrary, name: &str) {
    let key = match &instance.model_key {
        Some(key) => key,
        None => return,
    };
    if let Some(model) = library.get(key) {
        if let Some(index) = model.sequence_index_by_name(name) {
            instance.sequence = index;
        }
    }
}

/// 应用一个已去重的事件到实例上；纯表现层副作用。
fn apply_event(instance: &mut Instance, event: &Event, library: &ModelLibrary) {
    match event.kind {
        EventKind::Spawned => {
            // 实例已由视图同步创建，这里保持幂等。
        }
        EventKind::Died => switch_sequence(instance, library, "death"),
        EventKind::AttackHit => switch_sequence(instance, library, "attack"),
        EventKind::Arrived | EventKind::Stopped => {
            switch_sequence(instance, library, "stand");
        }
        EventKind::MoveFailed => {}
    }
}

fn surface_color(vertex: &TerrainVertex) -> [f32; 4] {
    if vertex.water > 0.0 {
        // 水体：按导出 waterHeights 呈现。
        return [0.15, 0.35, 0.75, 0.85];
    }
    if vertex.cliff > 0.0 {
        // 悬崖层：按导出 cliffTextures 呈现。
        return [0.45, 0.40, 0.35, 1.0];
    }
    // 地表：按 groundTextures 索引生成稳定颜色（不用随机源，保证 readback 可复现）。
    let shade = (vertex.surface * 0.137).fract();
    [0.20 + shade * 0.25, 0.45 + shade * 0.20, 0.20, 1.0]
}

fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
    [n[0] / len, n[1] / len, n[2] / len]
}

/// 单位占位标记（四面体）的基准尺寸；拾取平面高度由它推导。
pub const UNIT_MARKER_SIZE: f32 = 24.0;

/// 追加一个朝向标记（四面体）作为单位占位呈现，贴地并应用队伍色。
fn append_unit_marker(vertices: &mut Vec<SceneVertex>, instance: &Instance, terrain: &TerrainMesh) {
    let base_y = terrain.height_at_world(instance.x, instance.y);
    let size = UNIT_MARKER_SIZE;
    let (sin, cos) = instance.facing.sin_cos();
    let forward = [cos, sin];
    let apex = [instance.x, base_y + size * 1.6, instance.y];
    let front = [
        instance.x + forward[0] * size,
        base_y,
        instance.y + forward[1] * size,
    ];
    let left = [
        instance.x - forward[0] * size * 0.5 - forward[1] * size * 0.6,
        base_y,
        instance.y - forward[1] * size * 0.5 + forward[0] * size * 0.6,
    ];
    let right = [
        instance.x - forward[0] * size * 0.5 + forward[1] * size * 0.6,
        base_y,
        instance.y - forward[1] * size * 0.5 - forward[0] * size * 0.6,
    ];
    let color = team_color(instance.team);
    for &(p0, p1, p2) in &[
        (apex, front, left),
        (apex, left, right),
        (apex, right, front),
        (front, right, left),
    ] {
        let normal = triangle_normal(p0, p1, p2);
        for position in [p0, p1, p2] {
            vertices.push(SceneVertex {
                position,
                normal,
                color,
                surface: -1.0,
            });
        }
    }
}

impl Scene {
    /// 消费一帧只读视图与事件，更新表现层实例映射。
    ///
    /// 权威来源是 `view.entities`（与事件是否丢失无关）；事件只用于动画
    /// 序列切换，并按 `sequence` 去重（`drain_events()` 可能重复投递）。
    /// 本函数不接触 `RtsMatch`，因此无法改变模拟推进。
    pub fn consume(&mut self, input: FrameInput<'_>, library: &ModelLibrary) {
        let view = input.view;

        // 1. 按只读视图同步实例（唯一权威来源）。
        let mut present = BTreeSet::new();
        for entity in &view.entities {
            present.insert(entity.id);
            let target_x = world_to_f32(entity.x);
            let target_y = world_to_f32(entity.y);
            if let Some(instance) = self.instances.get_mut(&entity.id) {
                instance.prev_x = instance.target_x;
                instance.prev_y = instance.target_y;
                instance.target_x = target_x;
                instance.target_y = target_y;
                instance.facing = facing_to_radians(entity.facing);
                instance.team = entity.owner;
                instance.hp = entity.hp;
                instance.alive = entity.alive;
            } else {
                self.instances.insert(
                    entity.id,
                    Instance {
                        id: entity.id,
                        x: target_x,
                        y: target_y,
                        prev_x: target_x,
                        prev_y: target_y,
                        target_x,
                        target_y,
                        facing: facing_to_radians(entity.facing),
                        team: entity.owner,
                        hp: entity.hp,
                        alive: entity.alive,
                        model_key: library.resolve(entity),
                        sequence: 0,
                    },
                );
            }
        }
        self.instances.retain(|id, _| present.contains(id));

        // 2. 事件消费（按 sequence 稳定排序 + 去重）。
        let mut events: Vec<&Event> = input.events.iter().collect();
        events.sort_by_key(|event| event.sequence);
        for event in events {
            if let Some(max) = self.max_sequence {
                if event.sequence <= max {
                    continue;
                }
            }
            if let Some(instance) = self.instances.get_mut(&event.entity) {
                apply_event(instance, event, library);
            }
            self.max_sequence = Some(match self.max_sequence {
                Some(max) => max.max(event.sequence),
                None => event.sequence,
            });
            self.applied_events += 1;
        }
        self.last_tick = view.tick;
    }

    /// 稳定顺序迭代所有实例（只读）。
    pub fn instances(&self) -> impl Iterator<Item = &Instance> {
        self.instances.values()
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub fn last_tick(&self) -> u64 {
        self.last_tick
    }

    pub fn max_sequence(&self) -> Option<u64> {
        self.max_sequence
    }

    pub fn applied_events(&self) -> u64 {
        self.applied_events
    }

    /// 对所有实例做视觉插值；只改表现层 `x` / `y`。
    pub fn interpolate_all(&mut self, alpha: f32) {
        for instance in self.instances.values_mut() {
            instance.interpolate(alpha);
        }
    }

    /// 生成一帧的非索引 GPU 顶点：地形三角形 + 可见单位标记。
    /// 纯函数，稳定顺序、无随机源，便于 GPU readback 逐帧比对。
    pub fn build_vertices(&self, terrain: &TerrainMesh) -> Vec<SceneVertex> {
        let mut vertices = Vec::with_capacity(terrain.indices.len() + self.instances.len() * 12);
        for &index in &terrain.indices {
            let vertex = &terrain.vertices[index as usize];
            vertices.push(SceneVertex {
                position: vertex.position,
                normal: vertex.normal,
                color: surface_color(vertex),
                surface: vertex.surface,
            });
        }
        for instance in self.instances.values() {
            if instance.alive && instance.model_key.is_some() {
                append_unit_marker(&mut vertices, instance, terrain);
            }
        }
        vertices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_core::EntityView;

    fn entity(id: u64, owner: u8, x: i64) -> EntityView {
        EntityView {
            id,
            x,
            y: 0,
            facing: 0,
            hp: 100,
            owner,
            alive: true,
            has_target: false,
        }
    }

    fn view(tick: u64, entities: Vec<EntityView>) -> WorldView {
        WorldView {
            tick,
            rng: 0,
            entities,
        }
    }

    fn event(sequence: u64, entity: u64, kind: EventKind) -> Event {
        Event {
            frame: 0,
            sequence,
            entity,
            kind,
        }
    }

    fn terrain() -> TerrainMesh {
        let hf: w3_content::TerrainHeightfield = serde_json::from_str(
            r#"{"tilepointWidth":2,"tilepointHeight":2,"mapWidth":1,"mapHeight":1,
                "heights":[0,0,0,0]}"#,
        )
        .unwrap();
        TerrainMesh::from_heightfield(&hf).unwrap()
    }

    #[test]
    fn consume_builds_instances_from_read_only_view() {
        let mut scene = Scene::default();
        let view = view(1, vec![entity(1, 2, 65536), entity(2, 3, -131072)]);
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &ModelLibrary::default(),
        );
        assert_eq!(scene.len(), 2);
        assert_eq!(scene.last_tick(), 1);
        let ids: Vec<u64> = scene.instances().map(|i| i.id).collect();
        assert_eq!(ids, vec![1, 2]);
        let first = scene.instances().next().unwrap();
        assert!((first.target_x - 1.0).abs() < 1e-6);
        assert_eq!(first.team, 2);
        // 空模型库 → 资源缺失，单位不显示。
        assert_eq!(first.model_key, None);
    }

    #[test]
    fn despawned_entities_are_removed() {
        let mut scene = Scene::default();
        let library = ModelLibrary::default();
        scene.consume(
            FrameInput {
                view: &view(1, vec![entity(1, 0, 0), entity(2, 0, 0)]),
                events: &[],
            },
            &library,
        );
        assert_eq!(scene.len(), 2);
        scene.consume(
            FrameInput {
                view: &view(2, vec![entity(1, 0, 0)]),
                events: &[],
            },
            &library,
        );
        assert_eq!(scene.len(), 1);
    }

    #[test]
    fn events_are_deduplicated_by_sequence() {
        let mut scene = Scene::default();
        let view = view(1, vec![entity(1, 0, 0)]);
        let events = vec![
            event(1, 1, EventKind::AttackHit),
            event(2, 1, EventKind::Arrived),
        ];
        let library = ModelLibrary::default();
        scene.consume(
            FrameInput {
                view: &view,
                events: &events,
            },
            &library,
        );
        assert_eq!(scene.applied_events(), 2);
        assert_eq!(scene.max_sequence(), Some(2));

        // 重复投递同一批：全部被去重。
        scene.consume(
            FrameInput {
                view: &view,
                events: &events,
            },
            &library,
        );
        assert_eq!(scene.applied_events(), 2);

        // 新 sequence 才被应用。
        scene.consume(
            FrameInput {
                view: &view,
                events: &[event(3, 1, EventKind::Died)],
            },
            &library,
        );
        assert_eq!(scene.applied_events(), 3);
        assert_eq!(scene.max_sequence(), Some(3));
    }

    #[test]
    fn interpolation_only_moves_visual_position() {
        let mut scene = Scene::default();
        scene.consume(
            FrameInput {
                view: &view(1, vec![entity(1, 0, 0)]),
                events: &[],
            },
            &ModelLibrary::default(),
        );
        // 制造一次位置推进。
        scene.consume(
            FrameInput {
                view: &view(2, vec![entity(1, 0, 131072)]),
                events: &[],
            },
            &ModelLibrary::default(),
        );
        let instance = scene.instances().next().unwrap();
        assert_eq!(instance.prev_x, 0.0);
        assert_eq!(instance.target_x, 2.0);
        scene.interpolate_all(0.5);
        let instance = scene.instances().next().unwrap();
        assert!((instance.x - 1.0).abs() < 1e-6);
        // 逻辑目标不受插值影响。
        assert_eq!(instance.target_x, 2.0);
        assert_eq!(instance.prev_x, 0.0);
    }

    #[test]
    fn build_vertices_is_deterministic_and_skips_missing_models() {
        let mut library = ModelLibrary::default();
        library.insert("default", crate::model::Model::default());
        let mut scene = Scene::default();
        scene.consume(
            FrameInput {
                view: &view(1, vec![entity(1, 0, 65536)]),
                events: &[],
            },
            &library,
        );
        let mesh = terrain();
        let a = scene.build_vertices(&mesh);
        let b = scene.build_vertices(&mesh);
        assert_eq!(a, b, "顶点生成必须可复现");
        // 地形 6 索引 + 1 个单位标记 12 顶点 = 18。
        assert_eq!(a.len(), mesh.indices.len() + 12);
        assert!(a.iter().any(|v| v.surface < 0.0), "应含单位标记");
    }
}
