//! Coverage gate: every non-deferred registry pattern produces a real RN
//! component from `ReactNativeComponentRenderer::render_component`, never the
//! `null /* Unknown pattern: X */` sentinel that tsc cannot catch.
//!
//! Mirrors the spirit of crates/orbital-shell-android/tests/renderer_coverage.rs
//! for the mobile TypeScript shell.  The renderer lives in Rust (not Kotlin/Swift),
//! so the check is behavioural — we call the actual function under test — rather
//! than file-existence based.

use orbital_core::language::pattern_registry::get_component_mapping;
use orbital_shell_mobile::ReactNativeComponentRenderer;
use orbital_shell_typescript::{ComponentRenderer, TypeScriptBackend};
use std::collections::{HashMap, HashSet};

/// SVG/map/non-GPU canvas patterns deferred until platform drawing APIs are wired.
///
/// Game-category patterns (`battle-board`, `canvas-effect`, `graph-canvas`,
/// `positioned-canvas`, etc.) are NO LONGER deferred — they emit
/// `<OrbitalGameCanvas>` via the "game"/"canvas" arm in
/// `ReactNativeComponentRenderer`.  Only SVG primitives, map views, and
/// non-GPU drawing patterns remain here.
///
/// Must stay in sync with `DEFERRED_PATTERNS` in
/// `tools/almadar-pattern-sync/src/commands/android-renderers.ts`.
const DEFERRED_PATTERNS: &[&str] = &[
    "map-view",
    "scaled-diagram",
    "orbital-visualization",
    "jazari-state-machine",
    "signature-pad",
    "qr-scanner",
    "confetti-effect",
    "state-machine-view",
    "svg-branch",
    "svg-connection",
    "svg-flow",
    "svg-grid",
    "svg-lobe",
    "svg-mesh",
    "svg-morph",
    "svg-node",
    "svg-pulse",
    "svg-ring",
    "svg-shield",
    "svg-stack",
    "graph-view",
];

/// The `null /* Unknown pattern: X */` token emitted by
/// `ReactNativeComponentRenderer` when `get_component_mapping` returns `None`.
/// Must not appear in the output for any non-deferred registry pattern.
const UNKNOWN_PATTERN_SENTINEL: &str = "Unknown pattern:";

/// Read the installed `@almadar/core` patterns registry and return all non-deferred pattern names.
fn non_deferred_registry_patterns() -> Vec<String> {
    let registry: serde_json::Value = serde_json::from_str(
        &orbital_core::language_data::language_data()
            .unwrap_or_else(|e| panic!("{}: {e}", e.code()))
            .patterns,
    )
    .unwrap();

    let deferred: HashSet<&str> = DEFERRED_PATTERNS.iter().copied().collect();
    let patterns = registry["patterns"].as_object().unwrap();
    let mut result: Vec<String> = patterns
        .keys()
        .filter(|name| !deferred.contains(name.as_str()))
        .cloned()
        .collect();
    result.sort();
    result
}

/// Every non-deferred registry pattern must produce a non-null RN component
/// from `ReactNativeComponentRenderer` — never the `Unknown pattern` sentinel.
///
/// The only way this test can pass vacuously is if the installed registry is empty,
/// which would be caught by the `min_pattern_count` guard below.
#[test]
fn every_non_deferred_pattern_renders_real_rn_component() {
    let renderer = ReactNativeComponentRenderer;
    let backend = TypeScriptBackend::default();
    let config = HashMap::new();

    let patterns = non_deferred_registry_patterns();

    // Sanity: registry must be non-trivially populated so the test is not vacuous.
    assert!(
        patterns.len() >= 50,
        "mobile: installed registry looks empty or truncated — only {} non-deferred patterns; \
        run `make sync` to re-bake",
        patterns.len()
    );

    let mut failing: Vec<(String, String)> = Vec::new();
    for pattern in &patterns {
        let output = renderer.render_component(&backend, pattern, &config, None, None);
        if output.contains(UNKNOWN_PATTERN_SENTINEL) || output.trim() == "null" {
            failing.push((pattern.clone(), output));
        }
    }

    assert!(
        failing.is_empty(),
        "mobile: ReactNativeComponentRenderer emitted `null /* Unknown pattern */` for \
        {} registry pattern(s) — each needs an entry in \
        the installed `@almadar/core` component-mapping.json and a \
        category arm in `ReactNativeComponentRenderer::render_component`:\n{:#?}",
        failing.len(),
        failing
            .iter()
            .map(|(p, o)| format!("  {} → {}", p, o))
            .collect::<Vec<_>>()
    );
}

/// Every non-deferred pattern must have an entry in the installed component-mapping
/// (the data source that `get_component_mapping` reads).  A pattern absent from
/// the mapping falls through to the `Unknown pattern` null sentinel regardless
/// of category — this test catches that before the behavioural check above.
#[test]
fn every_non_deferred_pattern_has_component_mapping() {
    let patterns = non_deferred_registry_patterns();
    let mut missing = Vec::new();
    for pattern in &patterns {
        if get_component_mapping(pattern).is_none() {
            missing.push(pattern.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "mobile: {} pattern(s) are in the installed patterns-registry but missing from \
        component-mapping.json — `ReactNativeComponentRenderer` would emit \
        `null /* Unknown pattern */` for them (run `make sync` to re-bake):\n{:?}",
        missing.len(),
        missing
    );
}

// ── Negative tests — the gate must FAIL when a pattern has no handling ─────────

/// A pattern name that is guaranteed to be absent from the installed
/// component-mapping must produce the `Unknown pattern` sentinel.
///
/// This is the proof that the gate is live: if `render_component` stopped
/// returning the sentinel for unknown inputs (e.g. by adding a blanket
/// fallback), THIS test would break, alerting us that the coverage check
/// above is now vacuous.
#[test]
fn unknown_pattern_name_produces_sentinel_not_real_component() {
    let renderer = ReactNativeComponentRenderer;
    let backend = TypeScriptBackend::default();
    let config = HashMap::new();

    // This name intentionally does not appear in any installed registry or mapping.
    const FAKE_PATTERN: &str = "synthetic-test-only-no-such-pattern-xyz";

    // Pre-condition: confirm it really is absent.
    assert!(
        get_component_mapping(FAKE_PATTERN).is_none(),
        "mobile negative test: '{}' unexpectedly found in component-mapping — \
        choose a different sentinel name",
        FAKE_PATTERN
    );

    let output = renderer.render_component(&backend, FAKE_PATTERN, &config, None, None);

    assert!(
        output.contains(UNKNOWN_PATTERN_SENTINEL),
        "mobile: ReactNativeComponentRenderer did NOT produce the `Unknown pattern` \
        sentinel for an unregistered pattern name '{}' — the null-detection path \
        is broken; got: {:?}",
        FAKE_PATTERN,
        output
    );

    // Also confirm it is NOT a real RN component (must start with null, not JSX).
    assert!(
        output.trim().starts_with("null"),
        "mobile: output for unknown pattern should start with `null`, got: {:?}",
        output
    );
}

/// Removing a pattern from the component-mapping causes `get_component_mapping`
/// to return `None`, which triggers the `Unknown pattern` sentinel.  We simulate
/// this by calling the renderer directly with a pattern that was removed from
/// the mapping (using a name known to have been in the registry but
/// intentionally absent from the mapping in the test build — since we cannot
/// mutate the static mapping at test time, we rely on the `unknown_pattern`
/// negative test above as the proof of the detection path, and add this
/// doctest-style assertion to confirm the logic chain).
///
/// What this test actually verifies: if `every_non_deferred_pattern_has_component_mapping`
/// passes AND `unknown_pattern_name_produces_sentinel_not_real_component` passes,
/// then `every_non_deferred_pattern_renders_real_rn_component` is non-vacuous —
/// the sentinel is real and any mapping gap would be caught.
#[test]
fn gate_is_non_vacuous_sentinel_is_detectable() {
    // The three assertions below form the logical chain that makes the gate real.

    // 1. The sentinel string is the literal produced by the renderer for unknown inputs.
    let renderer = ReactNativeComponentRenderer;
    let backend = TypeScriptBackend::default();
    let config = HashMap::new();
    let sentinel_output =
        renderer.render_component(&backend, "no-such-pattern-gate-nonvacuous", &config, None, None);
    assert!(
        sentinel_output.contains("Unknown pattern:"),
        "sentinel detection broken: got {:?}",
        sentinel_output
    );

    // 2. The sentinel is NOT produced for `clear` (the one valid null return).
    let clear_output = renderer.render_component(&backend, "clear", &config, None, None);
    assert_eq!(
        clear_output, "null",
        "`clear` must return bare `null`, got {:?}",
        clear_output
    );
    assert!(
        !clear_output.contains("Unknown pattern:"),
        "`clear` must not trigger the unknown-pattern sentinel"
    );

    // 3. The sentinel IS different from the `clear` null — so the positive test
    //    would catch a regression where every pattern returned "null".
    assert_ne!(
        sentinel_output, clear_output,
        "sentinel output must differ from the `clear` null so the positive test \
        can distinguish them"
    );
}

/// Gate: every game-category pattern that was removed from DEFERRED must now
/// emit `<OrbitalGameCanvas` instead of the generic fallback or the null sentinel.
///
/// This is the coverage requirement for PART 2 of the mobile GPU module:
/// the RN ComponentRenderer "game"/"canvas" arm must route these patterns
/// to the native GPU view, not to a placeholder `<View>` or null.
#[test]
fn game_canvas_patterns_emit_orbital_game_canvas() {
    let renderer = ReactNativeComponentRenderer;
    let backend = TypeScriptBackend::default();
    let config = HashMap::new();

    // Canonical game-category patterns that must now render via OrbitalGameCanvas.
    // These were previously in DEFERRED_PATTERNS; after this change they are active.
    let game_patterns = &[
        "game-canvas-2d",
        "isometric-canvas",
        "platformer-canvas",
        "simulation-canvas",
        "positioned-canvas",
        "canvas-effect",
        "graph-canvas",
        "battle-board",
        "battle-template",
        "castle-board",
        "castle-template",
        "combat-log",
        "combo-counter",
        "builder-board",
        "classifier-board",
    ];

    let mut failing: Vec<(&str, String)> = Vec::new();
    for pattern in game_patterns {
        // Skip patterns not present in the installed registry (registry gap, not a renderer gap).
        if get_component_mapping(pattern).is_none() {
            continue;
        }
        let output = renderer.render_component(&backend, pattern, &config, None, None);
        if !output.contains("<OrbitalGameCanvas") {
            failing.push((pattern, output));
        }
    }

    assert!(
        failing.is_empty(),
        "mobile: {} game-category pattern(s) did NOT emit `<OrbitalGameCanvas` — \
        the \"game\"/\"canvas\" arm in ReactNativeComponentRenderer must route them \
        to the native GPU view:\n{}",
        failing.len(),
        failing
            .iter()
            .map(|(p, o)| format!("  {} → {:?}", p, o))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
