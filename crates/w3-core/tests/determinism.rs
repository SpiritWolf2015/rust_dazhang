//! P0 acceptance: two instances, snapshot round-trip and replay must agree
//! frame by frame. These are the determinism gates from the P0 checklist.

use w3_core::{
    Command, CommandKind, EntityId, Frame, MatchConfig, RtsMatch, SimFixed, SnapshotError,
};

fn config() -> MatchConfig {
    MatchConfig {
        seed: 0x5eed,
        logic_fps: 30,
        content_hash: 0xC0FFEE,
    }
}

/// A fixed command script. The two Spawn commands at frame 0 are ordered by
/// player id (1 then 2), so the kernel's never-reused ID counter gives them
/// IDs 1 and 2. Depending on that is intentional: it is part of the contract.
fn script() -> Vec<Command> {
    let first = EntityId::new(1).unwrap();
    let second = EntityId::new(2).unwrap();
    vec![
        Command::new(Frame::new(0), 1, 1, CommandKind::Spawn),
        Command::new(Frame::new(0), 2, 1, CommandKind::Spawn),
        Command::new(
            Frame::new(1),
            1,
            2,
            CommandKind::Move {
                entity: first,
                x: SimFixed::from_int(5).unwrap(),
                y: SimFixed::from_int(3).unwrap(),
            },
        ),
        Command::new(
            Frame::new(1),
            2,
            2,
            CommandKind::Move {
                entity: second,
                x: SimFixed::from_int(-4).unwrap(),
                y: SimFixed::from_int(2).unwrap(),
            },
        ),
        Command::new(Frame::new(6), 1, 3, CommandKind::Stop { entity: first }),
        Command::new(
            Frame::new(12),
            2,
            3,
            CommandKind::Move {
                entity: second,
                x: SimFixed::from_int(9).unwrap(),
                y: SimFixed::from_int(9).unwrap(),
            },
        ),
    ]
}

fn seeded(script: &[Command]) -> RtsMatch {
    let mut sim = RtsMatch::create(config());
    for command in script {
        sim.submit_command(*command).unwrap();
    }
    sim
}

#[test]
fn two_instances_interleaved_match_frame_hashes() {
    let script = script();
    let mut a = seeded(&script);
    let mut b = seeded(&script);
    let mut hashes = Vec::new();

    for frame in 0..40 {
        // Interleave the two instances so neither can share hidden state.
        if frame % 2 == 0 {
            a.step();
            b.step();
        } else {
            b.step();
            a.step();
        }
        let hash_a = a.compute_state_hash();
        let hash_b = b.compute_state_hash();
        assert_eq!(
            hash_a,
            hash_b,
            "frame {frame} diverged at {:?}",
            a.first_difference(&b)
        );
        hashes.push(hash_a);
    }

    // Guard against a vacuous pass where nothing ever changes.
    assert!(
        hashes.windows(2).any(|pair| pair[0] != pair[1]),
        "the simulation did not actually advance"
    );
}

#[test]
fn snapshot_roundtrip_continues_identically() {
    let script = script();
    let mut original = seeded(&script);
    for _ in 0..10 {
        original.step();
    }

    let snapshot = original.capture_snapshot();
    let mut restored = RtsMatch::restore_snapshot(snapshot.clone(), config().content_hash).unwrap();
    assert_eq!(original.compute_state_hash(), restored.compute_state_hash());
    assert_eq!(original.read_view(), restored.read_view());

    let mut reference = seeded(&script);
    for _ in 0..20 {
        reference.step();
    }
    for _ in 0..10 {
        original.step();
        restored.step();
    }
    assert_eq!(
        reference.compute_state_hash(),
        original.compute_state_hash()
    );
    assert_eq!(
        reference.compute_state_hash(),
        restored.compute_state_hash()
    );
}

#[test]
fn replay_of_recorded_commands_matches_original() {
    let recorded = script();
    let mut original = seeded(&recorded);
    let expected: Vec<u64> = (0..30)
        .map(|_| {
            original.step();
            original.compute_state_hash()
        })
        .collect();

    let mut replay = seeded(&recorded);
    for (frame, hash) in expected.iter().enumerate() {
        replay.step();
        assert_eq!(
            replay.compute_state_hash(),
            *hash,
            "replay diverged at frame {frame}"
        );
    }
}

#[test]
fn first_difference_names_the_changed_field() {
    let mut a = RtsMatch::create(config());
    let b = RtsMatch::create(config());
    a.step();
    assert_eq!(a.first_difference(&b), Some("tick"));
}

#[test]
fn snapshot_rejects_mismatched_content() {
    let mut sim = RtsMatch::create(config());
    sim.step();
    let snapshot = sim.capture_snapshot();
    assert!(matches!(
        RtsMatch::restore_snapshot(snapshot, 0xDEAD_BEEF),
        Err(SnapshotError::ContentMismatch)
    ));
}
