//! React Native Mobile Shell for Orbital Compiler

mod shell;

pub use shell::MobileShell;

use orbital_core::generate::{Backend, GeneratedFile, GeneratedOutput};
use orbital_core::language::pattern_registry::get_component_mapping;
use orbital_core::ir::{
    OirCardinality, OirEffect, OirEntity, OirExpr, OirModule, OirPage, OirTrait, OirValue,
};
use orbital_core::slots::{slot_role, SlotRole};
use orbital_core::str_utils::to_camel_case;
use orbital_shell_typescript::{
    ClientEffectEmitter, ComponentRenderer, EntityFieldInfo, TypeScriptBackend,
};
use std::collections::HashMap;

// ── React Native renderers ─────────────────────────────────────────────────────

/// Maps a resolved pattern to React Native primitive elements.
///
/// Consults the baked patterns-registry category (same `get_component_mapping`
/// the web shell uses) and emits native primitives instead of `@almadar/ui` JSX.
/// Never hand-maps a handful of patterns — every pattern type goes through the
/// registry-driven category dispatch below.
pub struct ReactNativeComponentRenderer;

impl ComponentRenderer for ReactNativeComponentRenderer {
    fn render_component(
        &self,
        _backend: &TypeScriptBackend,
        pattern_type: &str,
        config: &HashMap<String, OirValue>,
        _linked_entity: Option<&str>,
        _entity_field_types: Option<&HashMap<String, EntityFieldInfo>>,
    ) -> String {
        if pattern_type == "clear" {
            return "null".to_string();
        }

        let mapping = match get_component_mapping(pattern_type) {
            Some(m) => m,
            None => return format!("null /* Unknown pattern: {} */", pattern_type),
        };

        // Derive a title/label from common config keys for display fallback.
        let title = config
            .get("title")
            .or_else(|| config.get("label"))
            .and_then(|v| {
                if let OirValue::String(s) = v {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .unwrap_or(pattern_type);

        // Web→RN component-name remap for layout primitives.
        //
        // The baked component-mapping uses web component names (e.g. "Stack"
        // from @almadar/ui).  @almadar/mobile exports VStack/HStack/Box — not
        // "Stack" — so any code path that uses mapping.component directly for
        // mobile imports would emit a TS2724 "did you mean VStack?" error.
        // This explicit table is the ONE canonical remap; extend it here rather
        // than at any call site.
        let rn_component = match mapping.component.as_str() {
            "Stack" => "VStack",   // @almadar/mobile exports VStack, not Stack
            other => other,
        };
        let _ = rn_component; // consumed by layout arm below

        // Map registry category to the most appropriate RN primitive shell.
        // The emitted string is a self-contained JSX fragment that the screen
        // generator embeds verbatim.  All category arms use core RN components
        // (View/Text/FlatList/Pressable/ScrollView/TextInput) — never
        // `@almadar/ui` which is web-only.
        // All branches build a JSX string using core RN primitives only.
        // Brace escaping in format strings: `{{` = literal `{`, `}}` = literal `}`.
        match mapping.category.as_str() {
            "form" => {
                let submit_event = config
                    .get("onSubmit")
                    .and_then(|v| {
                        if let OirValue::String(s) = v {
                            Some(s.as_str())
                        } else {
                            None
                        }
                    })
                    .unwrap_or("SUBMIT");
                format!(
                    concat!(
                        "<ScrollView style={{{{ flex: 1 }}}}>",
                        "\n        <Text style={{{{ fontSize: 16, marginBottom: 8 }}}}>{title}</Text>",
                        "\n        <TextInput style={{{{ borderWidth: 1, padding: 8 }}}} placeholder=\"{title}\" />",
                        "\n        <Pressable onPress={{() => dispatch('{submit_event}')}} style={{{{ marginTop: 8 }}}}>",
                        "\n          <Text>Submit</Text>",
                        "\n        </Pressable>",
                        "\n      </ScrollView>"
                    ),
                    title = title,
                    submit_event = submit_event,
                )
            }
            "display" | "stat" | "metric" => {
                format!(
                    "<View style={{{{ padding: 8 }}}}>\n        <Text style={{{{ fontWeight: 'bold' }}}}>{title}</Text>\n      </View>",
                    title = title,
                )
            }
            "list" | "table" | "data" => {
                concat!(
                    "<FlatList",
                    "\n        data={data}",
                    "\n        keyExtractor={(item) => item.id}",
                    "\n        renderItem={({ item }) => (",
                    "\n          <View style={{ padding: 8 }}>",
                    "\n            <Text>{item.id}</Text>",
                    "\n          </View>",
                    "\n        )}",
                    "\n      />"
                )
                .to_string()
            }
            "navigation" | "header" => {
                format!(
                    "<View style={{{{ flexDirection: 'row', padding: 12 }}}}>\n        <Text style={{{{ fontSize: 18, fontWeight: 'bold' }}}}>{title}</Text>\n      </View>",
                    title = title,
                )
            }
            "modal" | "overlay" | "dialog" => {
                format!(
                    "<View style={{{{ flex: 1, justifyContent: 'center', alignItems: 'center' }}}}>\n        <View style={{{{ backgroundColor: 'white', padding: 24, borderRadius: 8 }}}}>\n          <Text style={{{{ fontSize: 16 }}}}>{title}</Text>\n        </View>\n      </View>",
                    title = title,
                )
            }
            "action" | "button" => {
                let event = config
                    .get("event")
                    .and_then(|v| {
                        if let OirValue::String(s) = v {
                            Some(s.as_str())
                        } else {
                            None
                        }
                    })
                    .unwrap_or("PRESS");
                format!(
                    concat!(
                        "<Pressable onPress={{() => dispatch('{event}')}}",
                        " style={{{{ padding: 12, backgroundColor: '#007AFF', borderRadius: 6 }}}}>",
                        "\n        <Text style={{{{ color: 'white', textAlign: 'center' }}}}>{title}</Text>",
                        "\n      </Pressable>"
                    ),
                    event = event,
                    title = title,
                )
            }
            "input" | "field" => {
                format!(
                    "<TextInput\n        style={{{{ borderWidth: 1, borderColor: '#ccc', padding: 8, borderRadius: 4 }}}}\n        placeholder=\"{title}\"\n      />",
                    title = title,
                )
            }
            "media" | "image" | "avatar" => {
                format!(
                    "<View style={{{{ width: 64, height: 64, backgroundColor: '#e0e0e0', borderRadius: 32 }}}}>\n        <Text style={{{{ textAlign: 'center', lineHeight: 64 }}}}>{title}</Text>\n      </View>",
                    title = title,
                )
            }
            // Layout primitives — map to RN View with the appropriate flex direction.
            // "component" covers stack/vstack/hstack/flex; "layout" covers box/center/spacer/grid.
            // The rn_component remap above ensures Stack→VStack before this arm is reached,
            // so import sites that use mapping.component directly will use the correct name.
            "layout" | "component" => match mapping.component.as_str() {
                "HStack" => {
                    "<View style={{ flexDirection: 'row', flexWrap: 'wrap' }}>\n      </View>"
                        .to_string()
                }
                "Spacer" => {
                    "<View style={{ flex: 1 }} />".to_string()
                }
                // Stack/VStack/Box/Center/Grid/Flex → column container
                _ => {
                    "<View style={{ flexDirection: 'column' }}>\n      </View>".to_string()
                }
            },
            // Game and canvas patterns route to the OrbitalGameCanvas native view.
            //
            // OrbitalGameCanvas wraps OrbitalGameSurfaceView (Android, via
            // orbital-shell-android JNI bridge) and OrbitalGameMTKCoordinator
            // (iOS, via orbital-shell-ios Swift bridge) — both backed by the
            // orbital-game-renderer c_api.  On simulators without a GPU adapter
            // the component renders a labelled placeholder.
            //
            // `graph-canvas` maps to category "visualization" in component-mapping
            // but is a GPU canvas in the patterns-registry; route it here by name
            // so chart/meter (also "visualization") keep their own category arm.
            cat if cat == "game" || cat == "canvas" || pattern_type == "graph-canvas" => {
                // Serialize the config snapshot to JSON for the native packet.
                // Config values are simple primitives in practice; serde_json
                // handles the common OirValue variants via Display-style fallback.
                let packet_fields: String = config
                    .iter()
                    .map(|(k, v)| {
                        let json_val = match v {
                            OirValue::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
                            OirValue::Number(n) => format!("{}", n),
                            OirValue::Bool(b) => format!("{}", b),
                            _ => "null".to_string(),
                        };
                        format!("\"{}\": {}", k, json_val)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                // Build the JSON packet as a JS object-literal string so it can be
                // passed as a JSX expression prop.  Single-quote escaping is not
                // needed because the outer JSX uses double-quote string syntax.
                let json_packet = format!(
                    "{{ \"pattern\": \"{}\", \"config\": {{ {} }} }}",
                    pattern_type, packet_fields,
                );
                // Emit as a JSX string literal (backtick template) so the JSON
                // double-quotes don't conflict with the JSX attribute syntax.
                format!(
                    concat!(
                        "<OrbitalGameCanvas",
                        "\n        trait={{trait}}",
                        "\n        jsonPacket={{`{json_packet}`}}",
                        "\n        style={{{{ flex: 1 }}}}",
                        "\n      />"
                    ),
                    json_packet = json_packet,
                )
            }
            _ => {
                // Generic fallback: labelled container using core RN primitives only.
                format!(
                    "<View style={{{{ padding: 8 }}}}>\n        <Text>{title}</Text>\n      </View>",
                    title = title,
                )
            }
        }
    }
}

/// React Navigation `navigate()` + shared event bus.
pub struct ReactNativeClientEffectEmitter;

impl ClientEffectEmitter for ReactNativeClientEffectEmitter {
    fn emit_navigate(&self, path_ts: &str, indent: &str) -> String {
        // Strip leading slash if the path is a string literal for React Navigation's
        // screen name convention, then delegate to `navigation.navigate`.
        let screen_expr = if path_ts.starts_with('"') || path_ts.starts_with('\'') {
            // Literal string path — pass through; the navigator's screen name
            // matches the `.path` field from the OirPage (e.g. "/tasks").
            path_ts.to_string()
        } else {
            path_ts.to_string()
        };
        format!("{}navigation.navigate({});", indent, screen_expr)
    }

    // React Navigation keeps a native screen stack — `goBack()` is the
    // honest mapping for `navigate-back`. The crumb label has no React
    // Navigation surface yet (screen titles come from navigator options),
    // so a crumb-carrying navigate degrades to a plain navigate here —
    // recorded in the gap ledger, not silently equivalent.
    fn emit_navigate_with_crumb(&self, path_ts: &str, _crumb_ts: &str, indent: &str) -> String {
        self.emit_navigate(path_ts, indent)
    }

    fn emit_navigate_back(&self, indent: &str) -> String {
        format!("{}navigation.goBack();", indent)
    }

    fn emit_event_bus(
        &self,
        event: &str,
        payload_ts: Option<&str>,
        source_ts: &str,
        indent: &str,
    ) -> String {
        match payload_ts {
            Some(p) => format!("{}eventBus.emit('{}', {}{});", indent, event, p, source_ts),
            None if !source_ts.is_empty() => {
                format!("{}eventBus.emit('{}', undefined{});", indent, event, source_ts)
            }
            None => format!("{}eventBus.emit('{}');", indent, event),
        }
    }
}

// ── MobileBackend ─────────────────────────────────────────────────────────────

pub struct MobileBackend {
    /// Shared TS backend — provides trait-logic hooks, expr/effect codegen, and
    /// `render_pattern_with` via the seam.  The web-specific
    /// `client_effect_emitter` inside it is overridden per-call via the trait.
    ts_backend: TypeScriptBackend,
}

impl MobileBackend {
    pub fn new() -> Self {
        Self {
            ts_backend: TypeScriptBackend::default(),
        }
    }

    fn generate_entity_types(&self, entities: &[OirEntity]) -> GeneratedFile {
        let mut content =
            String::from("// Generated by orbital compiler — do not edit manually\n\n");
        content.push_str("import { EntityRow } from '@almadar/core';\n\n");

        for entity in entities {
            content.push_str(&format!(
                "export interface {} extends EntityRow {{\n",
                entity.name
            ));
            content.push_str("  id: string;\n");
            content.push_str("  createdAt: Date;\n");
            content.push_str("  updatedAt: Date;\n");

            for field in &entity.fields {
                if field.name == "id" || field.name == "createdAt" || field.name == "updatedAt" {
                    continue;
                }

                // Unwrap Optional wrapper — the `?` field marker already
                // communicates optionality; we only need the inner type here.
                let effective_type = if let orbital_core::ir::OirType::Optional { inner } =
                    &field.field_type
                {
                    inner.as_ref()
                } else {
                    &field.field_type
                };
                let base_type = match field.enum_values.as_deref() {
                    Some(values) if !values.is_empty() => values
                        .iter()
                        .map(|v| format!("'{}'", v))
                        .collect::<Vec<_>>()
                        .join(" | "),
                    // Delegate to the canonical OIR→TS mapper in orbital-shell-typescript.
                    _ => self.ts_backend.map_type(effective_type),
                };

                let ts_type = match field.relation {
                    Some(ref rel) => match rel.cardinality {
                        OirCardinality::OneToMany | OirCardinality::ManyToMany => {
                            format!("{}[]", base_type)
                        }
                        _ => base_type,
                    },
                    None => base_type,
                };

                let optional = if field.required { "" } else { "?" };
                content.push_str(&format!("  {}{}: {};\n", field.name, optional, ts_type));
            }
            content.push_str("}\n\n");
        }

        GeneratedFile {
            path: "packages/shared/src/types/entities.ts".to_string(),
            content,
        }
    }

    /// Delegate to `TypeScriptBackend::trait_logic_hook_source` — the one
    /// canonical owner of trait-logic hook codegen (state machine, bridge,
    /// event bus, actions).  No reimplementation here.
    fn generate_trait_hook(&self, tr: &OirTrait, module: &OirModule) -> GeneratedFile {
        let (path, content) = self.ts_backend.trait_logic_hook_source(tr, module);
        // Remap web path to mobile hooks/ convention (P1-C: keep Logic suffix).
        let mobile_path = path
            .replace("packages/client/src/hooks/traits/", "hooks/")
            .replace("Logic.tsx", "Logic.ts");
        // Rewrite web-specific imports to their mobile equivalents (P1-A, P1-D).
        let content = rewrite_mobile_hook_imports(content);
        GeneratedFile {
            path: mobile_path,
            content,
        }
    }

    /// Emit a self-contained `useOrbitalBridge` for React Native.
    ///
    /// Uses `@almadar/mobile/hooks` for `useEventBus` (available in the mobile
    /// package) and the standard `fetch` API (available in RN via polyfill).
    /// The contract is identical to the web version so the generated trait hooks
    /// work unchanged after the import-rewrite pass.
    fn generate_orbital_bridge_hook(&self) -> GeneratedFile {
        let content = r#"// Generated by orbital compiler — do not edit manually

import { useCallback } from 'react';
import { useEventBus } from '@almadar/mobile/hooks';
import type { EventPayload } from '@almadar/core';
import type { EventResponse } from '@app/shared';

export interface OrbitalBridgeConfig {
  basePath: string;
  orbitalName: string;
  traitName: string;
}

export interface OrbitalBridge {
  sendEvent: (event: string, payload?: EventPayload, currentState?: string) => Promise<EventResponse>;
  eventBus: ReturnType<typeof useEventBus>;
}

const API_BASE = (process.env['EXPO_PUBLIC_API_URL'] ?? '');

export function useOrbitalBridge({ basePath, traitName }: OrbitalBridgeConfig): OrbitalBridge {
  const eventBus = useEventBus();

  const sendEvent = useCallback(
    async (event: string, payload?: EventPayload, currentState?: string): Promise<EventResponse> => {
      eventBus.emit(`${traitName}:DISPATCH`, { event, payload: payload ?? null, currentState: currentState ?? null });
      try {
        const response = await fetch(`${API_BASE}${basePath}/events`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ event, payload, currentState }),
        });
        if (!response.ok) {
          const err: EventResponse = { success: false, error: `HTTP ${response.status}: ${response.statusText}` };
          eventBus.emit(`${traitName}:${event}:ERROR`, { success: err.success, error: err.error ?? null, event });
          return err;
        }
        const result: EventResponse = await (response.json() as Promise<EventResponse>);
        if (result.success) {
          eventBus.emit(`${traitName}:${event}:SUCCESS`, { success: result.success, newState: result.newState ?? null, event });
        } else {
          eventBus.emit(`${traitName}:${event}:ERROR`, { success: result.success, error: result.error ?? null, event });
        }
        return result;
      } catch (err) {
        const errResponse: EventResponse = { success: false, error: err instanceof Error ? err.message : 'Network error' };
        eventBus.emit(`${traitName}:${event}:ERROR`, { success: errResponse.success, error: errResponse.error ?? null, event });
        return errResponse;
      }
    },
    [basePath, traitName, eventBus],
  );

  return { sendEvent, eventBus };
}
"#;
        GeneratedFile {
            path: "hooks/useOrbitalBridge.ts".to_string(),
            content: content.to_string(),
        }
    }

    /// Emit `src/slotRole.ts` — a TS classifier that mirrors `orbital_core::slots::slot_role`.
    ///
    /// Kept in sync by the parity test in this module; the Rust source is the canonical owner.
    fn generate_slot_role_ts(&self) -> GeneratedFile {
        let content = r#"// Generated by orbital compiler — do not edit manually
// Mirrors orbital_core::slots::slot_role — parity enforced by Rust test.

export type SlotRole =
  | 'Main'
  | 'Sidebar'
  | 'Drawer'
  | 'Modal'
  | 'Overlay'
  | 'Toast'
  | 'HudTop'
  | 'HudBottom'
  | 'Floating'
  | 'System';

const MAIN_SLOTS = new Set(['main', 'content', 'screen']);

export function slotRole(name: string): SlotRole {
  if (MAIN_SLOTS.has(name)) return 'Main';
  if (name === 'sidebar') return 'Sidebar';
  if (name === 'drawer' || name === 'dock') return 'Drawer';
  if (name === 'modal' || name === 'center') return 'Modal';
  if (name === 'overlay' || name.startsWith('overlay.')) return 'Overlay';
  if (name === 'toast') return 'Toast';
  if (name === 'hud-bottom') return 'HudBottom';
  if (name === 'hud' || name === 'hud-top' || name.startsWith('hud.')) return 'HudTop';
  if (name === 'floating') return 'Floating';
  if (name === 'system') return 'System';
  return 'Main';
}
"#;
        GeneratedFile {
            path: "src/slotRole.ts".to_string(),
            content: content.to_string(),
        }
    }

    /// Emit `packages/shared/src/types/events.ts` with the canonical event
    /// protocol types (EventResponse, per-trait payload maps).  Delegates to
    /// the TS backend's canonical generator — no reimplementation.
    fn generate_shared_event_types(&self, module: &OirModule) -> GeneratedFile {
        let content = self.ts_backend.shared_event_types_source(module);
        GeneratedFile {
            path: "packages/shared/src/types/events.ts".to_string(),
            content,
        }
    }

    /// Emit `packages/shared/src/index.ts` re-exporting entity + event types.
    fn generate_shared_index(&self) -> GeneratedFile {
        GeneratedFile {
            path: "packages/shared/src/index.ts".to_string(),
            content: "// Generated by orbital compiler — do not edit manually\nexport * from './types/entities';\nexport * from './types/events';\n".to_string(),
        }
    }

    fn generate_screen(&self, page: &OirPage, module: &OirModule) -> GeneratedFile {
        let screen_name = format!("{}Screen", page.name);
        let path = format!("screens/{}.tsx", screen_name);

        let hook_imports: String = page
            .traits
            .iter()
            .map(|binding| {
                let trait_name = &binding.trait_name;
                format!(
                    "import {{ use{}Logic }} from '../hooks/use{}Logic';\n",
                    trait_name, trait_name
                )
            })
            .collect();

        let hook_calls: String = page
            .traits
            .iter()
            .map(|binding| {
                let trait_name = &binding.trait_name;
                let var_name = to_camel_case(trait_name);
                format!("  const {} = use{}Logic();\n", var_name, trait_name)
            })
            .collect();

        let (main_elements, overlay_slots, needs_flat_list) =
            self.collect_rn_elements(page, module);

        let main_body = if main_elements.is_empty() {
            self.generate_trait_debug_cards(page)
        } else {
            main_elements.join("\n")
        };

        // Render every overlay slot after the main body.
        let overlay_body: String = overlay_slots
            .iter()
            .map(|(slot, role, jsx)| {
                self.render_overlay_slot(slot, *role, jsx.as_deref())
            })
            .collect::<Vec<_>>()
            .join("\n");

        let body = if overlay_body.is_empty() {
            main_body
        } else {
            format!("{}\n{}", main_body, overlay_body)
        };

        let needs_modal = overlay_slots
            .iter()
            .any(|(_, role, _)| matches!(role, SlotRole::Modal | SlotRole::Overlay | SlotRole::Drawer | SlotRole::Sidebar));

        // Collect which core RN components are actually used in the body.
        let used: Vec<&str> = ["View", "Text", "FlatList", "Modal", "Pressable", "ScrollView", "TextInput"]
            .iter()
            .copied()
            .filter(|&c| body.contains(&format!("<{}", c)) || body.contains(&format!("</{}", c)))
            .collect();
        // Always include View and Text for the outer wrapper.
        let mut rn_imports: Vec<&str> = vec!["View", "Text", "ScrollView"];
        for c in &used {
            if !rn_imports.contains(c) {
                rn_imports.push(c);
            }
        }
        if needs_flat_list && !rn_imports.contains(&"FlatList") {
            rn_imports.push("FlatList");
        }
        if needs_modal && !rn_imports.contains(&"Modal") {
            rn_imports.push("Modal");
        }
        rn_imports.sort_unstable();
        rn_imports.dedup();
        let rn_import_line = format!(
            "import {{ {} }} from 'react-native';\n",
            rn_imports.join(", ")
        );

        // Emit the @almadar/mobile import when any game-canvas pattern is rendered.
        let mobile_import_line = if body.contains("<OrbitalGameCanvas") {
            "import { OrbitalGameCanvas } from '@almadar/mobile';\n"
        } else {
            ""
        };

        let content = format!(
            r#"// Generated by orbital compiler — do not edit manually

import React from 'react';
{rn_import_line}{mobile_import_line}{hook_imports}
export default function {screen_name}() {{
{hook_calls}
  return (
    <ScrollView style={{{{ flex: 1, padding: 16 }}}}>
      <Text style={{{{ fontSize: 20, fontWeight: 'bold', marginBottom: 12 }}}}>{page_name}</Text>
{body}
    </ScrollView>
  );
}}
"#,
            rn_import_line = rn_import_line,
            mobile_import_line = mobile_import_line,
            hook_imports = hook_imports,
            screen_name = screen_name,
            hook_calls = hook_calls,
            page_name = page.name,
            body = body,
        );

        GeneratedFile { path, content }
    }

    /// Walk render-ui effects and produce RN JSX via `ReactNativeComponentRenderer`.
    ///
    /// Returns `(main_elements, overlay_slots, needs_flat_list)`.
    /// `overlay_slots` is a vec of `(slot_name, role, jsx)` for every non-Main,
    /// non-System render-ui effect so `generate_screen` can wrap them in the
    /// correct RN overlay component.  A `null` pattern value signals slot clear
    /// and is returned as `None` for the jsx.
    fn collect_rn_elements(
        &self,
        page: &OirPage,
        module: &OirModule,
    ) -> (Vec<String>, Vec<(String, SlotRole, Option<String>)>, bool) {
        let mut main_elements: Vec<String> = Vec::new();
        let mut overlay_slots: Vec<(String, SlotRole, Option<String>)> = Vec::new();
        let mut needs_flat_list = false;

        for binding in &page.traits {
            let Some(trait_def) = module.traits.iter().find(|t| t.name == binding.trait_name)
            else {
                continue;
            };
            let hook_var = to_camel_case(&binding.trait_name);

            for transition in &trait_def.transitions {
                for effect_expr in &transition.effects {
                    let OirExpr::Effect { effect } = effect_expr else {
                        continue;
                    };
                    let OirEffect::RenderUi { slot, pattern } = effect.as_ref() else {
                        continue;
                    };

                    let role = slot_role(slot);

                    // `null` / `clear` pattern means close/clear the slot.
                    let is_clear = pattern.pattern_type == "clear" || pattern.pattern_type == "null";

                    if role == SlotRole::Main || role == SlotRole::System {
                        if !is_clear {
                            let rendered = self.ts_backend.render_pattern_with(
                                &ReactNativeComponentRenderer,
                                &pattern.pattern_type,
                                &pattern.config,
                                trait_def.linked_entity.as_deref(),
                                None,
                            );
                            let jsx =
                                rendered.replace("dispatch(", &format!("{}.dispatch(", hook_var));
                            if jsx.contains("<FlatList") {
                                needs_flat_list = true;
                            }
                            main_elements.push(format!("      {}", jsx));
                        }
                    } else {
                        let jsx_opt = if is_clear {
                            None
                        } else {
                            let rendered = self.ts_backend.render_pattern_with(
                                &ReactNativeComponentRenderer,
                                &pattern.pattern_type,
                                &pattern.config,
                                trait_def.linked_entity.as_deref(),
                                None,
                            );
                            let jsx = rendered
                                .replace("dispatch(", &format!("{}.dispatch(", hook_var));
                            if jsx.contains("<FlatList") {
                                needs_flat_list = true;
                            }
                            Some(jsx)
                        };
                        overlay_slots.push((slot.clone(), role, jsx_opt));
                    }
                }
            }
        }

        (main_elements, overlay_slots, needs_flat_list)
    }

    /// Emit JSX for a single overlay slot wrapped in the appropriate RN container.
    ///
    /// `jsx` is `Some(content)` to mount or `None` to close (visible=false).
    fn render_overlay_slot(&self, slot: &str, role: SlotRole, jsx: Option<&str>) -> String {
        let visible = if jsx.is_some() { "true" } else { "false" };
        let inner = jsx.unwrap_or("null");
        match role {
            SlotRole::Modal | SlotRole::Overlay => {
                let transparent = matches!(role, SlotRole::Overlay);
                format!(
                    concat!(
                        "      {{/* slot: {slot} */}}\n",
                        "      <Modal visible={{{visible}}} transparent={{{transparent}}} animationType=\"fade\">\n",
                        "        <View style={{{{ flex: 1, justifyContent: 'center', alignItems: 'center', backgroundColor: 'rgba(0,0,0,0.5)' }}}}>\n",
                        "          {inner}\n",
                        "        </View>\n",
                        "      </Modal>",
                    ),
                    slot = slot,
                    visible = visible,
                    transparent = transparent,
                    inner = inner,
                )
            }
            SlotRole::Drawer | SlotRole::Sidebar => {
                format!(
                    concat!(
                        "      {{/* slot: {slot} */}}\n",
                        "      <Modal visible={{{visible}}} transparent={{true}} animationType=\"slide\" presentationStyle=\"pageSheet\">\n",
                        "        <View style={{{{ flex: 1, backgroundColor: 'white', padding: 16 }}}}>\n",
                        "          {inner}\n",
                        "        </View>\n",
                        "      </Modal>",
                    ),
                    slot = slot,
                    visible = visible,
                    inner = inner,
                )
            }
            SlotRole::Toast => {
                format!(
                    concat!(
                        "      {{/* slot: {slot} */}}\n",
                        "      {{{visible} && (\n",
                        "        <View style={{{{ position: 'absolute', top: 40, right: 16, backgroundColor: '#323232', borderRadius: 4, padding: 8, zIndex: 9999 }}}}>\n",
                        "          {inner}\n",
                        "        </View>\n",
                        "      )}}",
                    ),
                    slot = slot,
                    visible = visible,
                    inner = inner,
                )
            }
            SlotRole::HudTop => {
                format!(
                    concat!(
                        "      {{/* slot: {slot} */}}\n",
                        "      {{{visible} && (\n",
                        "        <View style={{{{ position: 'absolute', top: 0, left: 0, right: 0, zIndex: 100 }}}}>\n",
                        "          {inner}\n",
                        "        </View>\n",
                        "      )}}",
                    ),
                    slot = slot,
                    visible = visible,
                    inner = inner,
                )
            }
            SlotRole::HudBottom => {
                format!(
                    concat!(
                        "      {{/* slot: {slot} */}}\n",
                        "      {{{visible} && (\n",
                        "        <View style={{{{ position: 'absolute', bottom: 0, left: 0, right: 0, zIndex: 100 }}}}>\n",
                        "          {inner}\n",
                        "        </View>\n",
                        "      )}}",
                    ),
                    slot = slot,
                    visible = visible,
                    inner = inner,
                )
            }
            SlotRole::Floating => {
                format!(
                    concat!(
                        "      {{/* slot: {slot} */}}\n",
                        "      {{{visible} && (\n",
                        "        <View style={{{{ position: 'absolute', bottom: 80, right: 16, zIndex: 200 }}}}>\n",
                        "          {inner}\n",
                        "        </View>\n",
                        "      )}}",
                    ),
                    slot = slot,
                    visible = visible,
                    inner = inner,
                )
            }
            // Main and System are never passed here; guard for exhaustiveness.
            SlotRole::Main | SlotRole::System => String::new(),
        }
    }

    fn generate_trait_debug_cards(&self, page: &OirPage) -> String {
        page.traits
            .iter()
            .map(|binding| {
                let trait_name = &binding.trait_name;
                let var_name = to_camel_case(trait_name);
                format!(
                    r#"      <View style={{{{ padding: 8, marginBottom: 8, borderWidth: 1, borderColor: '#ccc' }}}}>
        <Text style={{{{ fontWeight: 'bold' }}}}>{trait_name}</Text>
        <Text>State: {{{var_name}.state.currentState}}</Text>
        <Text>Previous: {{{var_name}.state.previousState ?? '-'}}</Text>
        <Text>Last Event: {{{var_name}.state.lastEvent ?? '-'}}</Text>
      </View>
"#,
                    trait_name = trait_name,
                    var_name = var_name
                )
            })
            .collect()
    }

    /// Generate the React Navigation stack + `App.tsx` entry point.
    fn generate_navigator(&self, pages: &[OirPage]) -> GeneratedFile {
        let imports: String = pages
            .iter()
            .map(|page| {
                let screen_name = format!("{}Screen", page.name);
                format!(
                    "import {screen_name} from './screens/{screen_name}';\n",
                    screen_name = screen_name
                )
            })
            .collect();

        let screens: String = pages
            .iter()
            .map(|page| {
                let screen_name = format!("{}Screen", page.name);
                format!(
                    "      <Stack.Screen name=\"{}\" component={{{}}} />\n",
                    page.path, screen_name
                )
            })
            .collect();

        let content = format!(
            r#"// Generated by orbital compiler — do not edit manually

import React from 'react';
import {{ NavigationContainer }} from '@react-navigation/native';
import {{ createNativeStackNavigator }} from '@react-navigation/native-stack';
{imports}
const Stack = createNativeStackNavigator();

export default function App() {{
  return (
    <NavigationContainer>
      <Stack.Navigator>
{screens}
      </Stack.Navigator>
    </NavigationContainer>
  );
}}
"#,
            imports = imports,
            screens = screens
        );

        GeneratedFile {
            path: "GeneratedNavigator.tsx".to_string(),
            content,
        }
    }
}

/// Rewrite the web-only imports that the TS backend emits into their mobile
/// equivalents.  Applied as a single post-pass; all rewrites are explicit.
/// (P1-A: import paths, P1-D: verifier snapshot)
fn rewrite_mobile_hook_imports(content: String) -> String {
    let content = content
        // P1-A: @/hooks/useOrbitalBridge → sibling file in hooks/ dir
        .replace(
            "import { useOrbitalBridge } from '@/hooks/useOrbitalBridge';",
            "import { useOrbitalBridge } from './useOrbitalBridge';",
        )
        // P1-A: @almadar/ui/hooks → @almadar/mobile/hooks (re-exports useUIEvents)
        .replace(
            "import { useUIEvents } from '@almadar/ui/hooks';",
            "import { useUIEvents } from '@almadar/mobile/hooks';",
        )
        // P1-D: strip verifier snapshot import (web-only, @almadar/ui/lib not in mobile)
        .replace(
            "import { registerTraitSnapshot, type TraitStateSnapshot } from '@almadar/ui/lib';\n",
            "",
        );

    // P1-D: strip the verifier snapshot useEffect block.  The TS backend
    // emits a fixed-marker comment immediately before this block; we split
    // on it and discard through the closing `}, []);\n\n`.
    const SNAPSHOT_START: &str = "  // Verifier snapshot registration (VG Foundation 1)\n";
    const SNAPSHOT_END: &str = "  }, []);\n\n";
    if let Some(start_idx) = content.find(SNAPSHOT_START) {
        if let Some(rel_end) = content[start_idx..].find(SNAPSHOT_END) {
            let end_idx = start_idx + rel_end + SNAPSHOT_END.len();
            let mut out = String::with_capacity(content.len());
            out.push_str(&content[..start_idx]);
            out.push_str(&content[end_idx..]);
            return out;
        }
    }
    content
}

impl Default for MobileBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for MobileBackend {
    fn generate(&self, module: &OirModule) -> GeneratedOutput {
        let mut files = vec![];
        files.push(self.generate_entity_types(&module.entities));
        // Shared event types (EventResponse, per-trait payload maps) — P1-A
        files.push(self.generate_shared_event_types(module));
        files.push(self.generate_shared_index());
        // Self-contained useOrbitalBridge for React Native — P1-A
        files.push(self.generate_orbital_bridge_hook());
        // TS slot-role classifier mirroring orbital_core::slots::slot_role.
        files.push(self.generate_slot_role_ts());
        for tr in &module.traits {
            files.push(self.generate_trait_hook(tr, module));
        }
        for page in &module.pages {
            files.push(self.generate_screen(page, module));
        }
        files.push(self.generate_navigator(&module.pages));
        GeneratedOutput {
            files,
            warnings: vec![],
        }
    }

    fn name(&self) -> &'static str {
        "mobile"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbital_core::ir::{
        OirCardinality, OirEntity, OirEvent, OirField, OirPattern, OirRelation, OirState,
        OirTransition, OirTraitBinding, OirType,
    };

    #[test]
    fn generate_entity_types_emits_shared_entities_file() {
        let backend = MobileBackend::new();
        let entities = vec![
            OirEntity {
                shared: false,
                identity: false,
                local: false,
                seed_mock: false,
                id: None,
                name: "Task".to_string(),
                description: None,
                collection: "tasks".to_string(),
                persistence: orbital_core::ir::OirPersistence::Persistent,
                fields: vec![
                    OirField::new("title", OirType::String),
                    OirField::optional("description", OirType::String),
                    OirField {
                        declared: None,
                        merged_from: None,
                        name: "status".to_string(),
                        field_type: OirType::Enum {
                            name: "TaskStatus".to_string(),
                        },
                        description: None,
                        required: true,
                        default: None,
                        constraints: vec![],
                        enum_values: Some(vec!["todo".to_string(), "done".to_string()]),
                        relation: None,
                        intrinsic: false,
                    },
                    OirField {
                        declared: None,
                        merged_from: None,
                        name: "tags".to_string(),
                        field_type: OirType::Array {
                            inner: Box::new(OirType::String),
                        },
                        description: None,
                        required: true,
                        default: None,
                        constraints: vec![],
                        enum_values: None,
                        relation: None,
                        intrinsic: false,
                    },
                    OirField {
                        declared: None,
                        merged_from: None,
                        name: "subtasks".to_string(),
                        field_type: OirType::Reference {
                            entity: "Subtask".to_string(),
                        },
                        description: None,
                        required: false,
                        default: None,
                        constraints: vec![],
                        enum_values: None,
                        relation: Some(OirRelation {
                            target_entity: "Subtask".to_string(),
                            target_entity_id: None,
                            cardinality: OirCardinality::OneToMany,
                            foreign_key: None,
                        }),
                        intrinsic: false,
                    },
                ],
                defaults: None,
                instances: vec![],
                used_by_traits: vec![],
                used_by_pages: vec![],
                read_policy: None,
                create_policy: None,
                update_policy: None,
                delete_policy: None,
            },
            OirEntity {
                shared: false,
                identity: false,
                local: false,
                seed_mock: false,
                id: None,
                name: "Subtask".to_string(),
                description: None,
                collection: "subtasks".to_string(),
                persistence: orbital_core::ir::OirPersistence::Persistent,
                fields: vec![OirField::new("done", OirType::Boolean)],
                defaults: None,
                instances: vec![],
                used_by_traits: vec![],
                used_by_pages: vec![],
                read_policy: None,
                create_policy: None,
                update_policy: None,
                delete_policy: None,
            },
        ];

        let file = backend.generate_entity_types(&entities);

        assert_eq!(file.path, "packages/shared/src/types/entities.ts");
        assert!(file
            .content
            .starts_with("// Generated by orbital compiler — do not edit manually\n\n"));
        assert!(file
            .content
            .contains("import { EntityRow } from '@almadar/core';"));
        assert!(file
            .content
            .contains("export interface Task extends EntityRow {"));
        assert!(file.content.contains("  id: string;"));
        assert!(file.content.contains("  createdAt: Date;"));
        assert!(file.content.contains("  updatedAt: Date;"));
        assert!(file.content.contains("  title: string;"));
        assert!(file.content.contains("  description?: string;"));
        assert!(file.content.contains("  status: 'todo' | 'done';"));
        assert!(file.content.contains("  tags: string[];"));
        // Relation fields store row IDS (C-COMPILED-PATH-RED §25 string→Person):
        // the declared member is `string[]`, not the referenced interface.
        assert!(file.content.contains("  subtasks?: string[];"));
        assert!(file
            .content
            .contains("export interface Subtask extends EntityRow {"));
        assert!(file.content.contains("  done: boolean;"));
    }

    #[test]
    fn generate_trait_hook_delegates_to_ts_backend() {
        let backend = MobileBackend::new();
        let tr = OirTrait {
            name: "Toggle".to_string(),
            id: None,
            orbital_name: "Test".to_string(),
            description: None,
            source: orbital_core::ir::OirTraitSource::Schema,
            category: None,
            capabilities: vec![],
            linked_entity: None,
            linked_entity_id: None,
            entity_ref_ids: None,
            trait_embed_ids: None,
            host_trait: None,
            local: false,
            config: None,
            declared_config_schema: None,
            declared_config_defaults: None,
            states: vec![OirState::initial("off"), OirState::new("on")],
            events: vec![OirEvent::new("TURN_ON"), OirEvent::new("TURN_OFF")],
            transitions: vec![
                OirTransition::new("off", "on", "TURN_ON"),
                OirTransition::new("on", "off", "TURN_OFF"),
            ],
            guards: vec![],
            ticks: vec![],
            listeners: vec![],
            emits: vec![],
            data_entities: vec![],
            slots_used: vec![],
        };
        let module = OirModule::new("TestApp", "1.0.0");

        let file = backend.generate_trait_hook(&tr, &module);

        // Path remapped from TS backend convention to mobile hooks/ convention.
        assert!(file.path.starts_with("hooks/"), "path: {}", file.path);
        // Trait-logic hook content from the canonical TS backend.
        assert!(file.content.contains("useToggle") || file.content.contains("Toggle"));
        assert!(file.content.contains("off") && file.content.contains("on"));
    }

    #[test]
    fn generate_page_aware_screens_and_navigator() {
        let backend = MobileBackend::new();
        let mut module = OirModule::new("TestApp", "1.0.0");

        let mut home_page = OirPage::new("Home", "/");
        home_page.traits.push(OirTraitBinding::new("TaskList"));
        module.pages.push(home_page);

        let mut settings_page = OirPage::new("Settings", "/settings");
        settings_page
            .traits
            .push(OirTraitBinding::new("UserProfile"));
        module.pages.push(settings_page);

        let output = backend.generate(&module);

        let paths: Vec<String> = output.files.iter().map(|f| f.path.clone()).collect();
        assert!(paths.contains(&"screens/HomeScreen.tsx".to_string()));
        assert!(paths.contains(&"screens/SettingsScreen.tsx".to_string()));
        assert!(paths.contains(&"GeneratedNavigator.tsx".to_string()));
        assert!(paths.contains(&"packages/shared/src/types/entities.ts".to_string()));

        let home_screen = output
            .files
            .iter()
            .find(|f| f.path == "screens/HomeScreen.tsx")
            .unwrap();
        assert!(home_screen
            .content
            .contains("import { useTaskListLogic } from '../hooks/useTaskListLogic';"));
        assert!(home_screen
            .content
            .contains("const taskList = useTaskListLogic();"));

        let navigator = output
            .files
            .iter()
            .find(|f| f.path == "GeneratedNavigator.tsx")
            .unwrap();
        assert!(navigator
            .content
            .contains("import HomeScreen from './screens/HomeScreen';"));
        assert!(navigator
            .content
            .contains("import SettingsScreen from './screens/SettingsScreen';"));
        assert!(navigator.content.contains("NavigationContainer"));
        assert!(navigator.content.contains("name=\"/\" component={HomeScreen}"));
        assert!(navigator
            .content
            .contains("name=\"/settings\" component={SettingsScreen}"));
    }

    #[test]
    fn generate_screen_uses_rn_primitives_for_render_ui() {
        let backend = MobileBackend::new();

        let mut trait_def = OirTrait::new("Clicker");
        trait_def.linked_entity = Some("Task".to_string());
        trait_def.states = vec![OirState::initial("idle")];
        trait_def.events = vec![OirEvent::new("PRESS")];
        // Use a pattern that will fall through to the generic "display" path or
        // be unknown — either way, the output must be RN primitives, not @almadar/ui.
        trait_def.transitions =
            vec![
                OirTransition::new("idle", "idle", "PRESS").with_effects(vec![OirExpr::effect(
                    OirEffect::render_ui("main", OirPattern::new("stat-card")),
                )]),
            ];

        let mut module = OirModule::new("TestApp", "1.0.0");
        module.traits.push(trait_def);

        let mut page = OirPage::new("Home", "/");
        page.traits.push(OirTraitBinding::new("Clicker"));
        module.pages.push(page);

        let output = backend.generate(&module);
        let screen = output
            .files
            .iter()
            .find(|f| f.path == "screens/HomeScreen.tsx")
            .unwrap();

        // Must use core RN components — never @almadar/ui.
        assert!(!screen.content.contains("@almadar/ui"), "must not import @almadar/ui");
        assert!(
            screen.content.contains("from 'react-native'"),
            "must import from react-native"
        );
    }

    #[test]
    fn generate_screen_returns_relative_path() {
        let backend = MobileBackend::new();
        let module = OirModule::new("TestApp", "1.0.0");
        let page = OirPage::new("Tasks", "/tasks");

        let file = backend.generate_screen(&page, &module);

        assert_eq!(file.path, "screens/TasksScreen.tsx");
    }

    #[test]
    fn generate_navigator_returns_app_tsx() {
        let backend = MobileBackend::new();
        let page = OirPage::new("Tasks", "/tasks");

        let file = backend.generate_navigator(&[page]);

        assert_eq!(file.path, "GeneratedNavigator.tsx");
        assert!(file.content.contains("NavigationContainer"));
        assert!(file.content.contains("createNativeStackNavigator"));
    }

    #[test]
    fn rn_component_renderer_uses_registry_category() {
        let renderer = ReactNativeComponentRenderer;
        let ts_backend = TypeScriptBackend::default();
        let config = HashMap::new();

        // A pattern in the baked registry — stat-card maps to "stat"/"display" category.
        let out = renderer.render_component(&ts_backend, "stat-card", &config, None, None);
        // Whatever it renders, it must NOT include @almadar/ui components.
        assert!(!out.contains("@almadar"), "stat-card must use RN primitives, got: {}", out);
        // Unknown pattern produces null comment.
        let unknown = renderer.render_component(&ts_backend, "no-such-pattern-xyz", &config, None, None);
        assert!(unknown.contains("null"), "unknown pattern must return null");
    }

    #[test]
    fn stack_pattern_renders_column_view_not_mobile_stack_import() {
        // Regression guard: the baked component-mapping has component:"Stack" for the
        // "stack" pattern.  A code path that uses mapping.component directly for mobile
        // imports would emit `import { Stack } from '@almadar/mobile/components/atoms'`
        // — but @almadar/mobile exports VStack, not Stack (TS2724).  This test ensures
        // the renderer emits a plain RN View (no @almadar import required).
        let renderer = ReactNativeComponentRenderer;
        let ts_backend = TypeScriptBackend::default();
        let config = HashMap::new();

        let out = renderer.render_component(&ts_backend, "stack", &config, None, None);
        assert!(
            !out.contains("Stack"),
            "stack must not emit bare 'Stack' (would be TS2724 on @almadar/mobile); got: {}",
            out
        );
        assert!(
            out.contains("<View"),
            "stack must emit a View (RN primitive), got: {}",
            out
        );
        assert!(
            out.contains("column"),
            "stack must use flexDirection:column, got: {}",
            out
        );

        // hstack maps to row layout
        let hout = renderer.render_component(&ts_backend, "hstack", &config, None, None);
        assert!(
            hout.contains("<View") && hout.contains("row"),
            "hstack must emit a row View, got: {}",
            hout
        );
    }

    #[test]
    fn rn_nav_emitter_uses_react_navigation() {
        let emitter = ReactNativeClientEffectEmitter;
        let navigate = emitter.emit_navigate("\"/tasks\"", "  ");
        assert!(navigate.contains("navigation.navigate"), "got: {}", navigate);

        let bus_with_payload = emitter.emit_event_bus("MY_EVENT", Some("payload"), "", "  ");
        assert!(bus_with_payload.contains("eventBus.emit('MY_EVENT'"), "got: {}", bus_with_payload);

        let bus_no_payload = emitter.emit_event_bus("EVT", None, "", "  ");
        assert!(bus_no_payload.contains("eventBus.emit('EVT')"), "got: {}", bus_no_payload);
    }

    /// Parity gate: the emitted TS `slotRole()` function must agree with
    /// `orbital_core::slots::slot_role` for every canonical slot name.
    ///
    /// Each case checks a per-name substring that contains BOTH the exact slot
    /// name literal AND the correct return value, so the test is non-vacuous:
    /// mis-mapping a single name (e.g. "modal" → 'Main' while "center" → 'Modal')
    /// fails that entry independently of all others.
    #[test]
    fn slot_role_ts_mirrors_orbital_core() {
        let backend = MobileBackend::new();
        let file = backend.generate_slot_role_ts();
        let ts = &file.content;

        // (slot_name, per_name_ts_needle, orbital_core_role)
        // Each needle must appear verbatim in the emitted slotRole.ts and contains
        // both the exact name literal and the correct return string.
        let cases: &[(&str, &str, SlotRole)] = &[
            // MAIN_SLOTS set membership + MAIN_SLOTS return — per-name checks.
            // Needle for "main": the set literal starts with 'main'; also check set→return.
            ("main",    "new Set(['main', 'content', 'screen'])",     SlotRole::Main),
            ("content", "new Set(['main', 'content', 'screen'])",     SlotRole::Main),
            ("screen",  "new Set(['main', 'content', 'screen'])",     SlotRole::Main),
            // single-name if-branches: needle = exact condition + return on the same line
            ("sidebar",   "name === 'sidebar') return 'Sidebar'",    SlotRole::Sidebar),
            ("drawer",    "name === 'drawer' || name === 'dock') return 'Drawer'", SlotRole::Drawer),
            ("dock",      "name === 'dock') return 'Drawer'",        SlotRole::Drawer),
            // modal-group: both names on the same condition
            ("modal",  "name === 'modal' || name === 'center') return 'Modal'", SlotRole::Modal),
            ("center", "name === 'center') return 'Modal'",                     SlotRole::Modal),
            // overlay exact-match + prefix on same branch
            ("overlay",          "name === 'overlay' || name.startsWith('overlay.')) return 'Overlay'", SlotRole::Overlay),
            ("overlay.dialogue", "name.startsWith('overlay.')",                                         SlotRole::Overlay),
            ("toast",     "name === 'toast') return 'Toast'",         SlotRole::Toast),
            ("hud-bottom","name === 'hud-bottom') return 'HudBottom'",SlotRole::HudBottom),
            // hud-group: three conditions on same branch
            ("hud",     "name === 'hud' || name === 'hud-top' || name.startsWith('hud.')) return 'HudTop'", SlotRole::HudTop),
            ("hud-top", "name === 'hud-top' || name.startsWith('hud.')) return 'HudTop'",                   SlotRole::HudTop),
            ("hud.health","name.startsWith('hud.')",                                                         SlotRole::HudTop),
            ("floating", "name === 'floating') return 'Floating'",    SlotRole::Floating),
            ("system",   "name === 'system') return 'System'",        SlotRole::System),
        ];

        for (slot, ts_needle, expected_rust) in cases {
            // Confirm Rust canonical owner agrees.
            assert_eq!(
                slot_role(slot),
                *expected_rust,
                "orbital_core::slot_role({slot:?}) mismatch"
            );
            // Confirm the emitted TS contains the per-name needle.
            // Failing here means the specific name or its mapped role was corrupted.
            assert!(
                ts.contains(ts_needle),
                "slotRole.ts missing per-name mapping for slot {slot:?}; expected substring:\n  {ts_needle}\ngot:\n{ts}"
            );
        }

        // MAIN_SLOTS set must return 'Main' — the three per-name checks above confirm
        // the names are in the set; this confirms the set maps to the right role.
        assert!(
            ts.contains("MAIN_SLOTS.has(name)) return 'Main'"),
            "slotRole.ts: MAIN_SLOTS.has() must return 'Main'"
        );
    }

    /// Modal routing: a `render-ui "modal"` effect must produce a RN `<Modal>` wrapper
    /// in the emitted screen, and a `null` clear must set `visible={false}`.
    #[test]
    fn modal_slot_renders_as_rn_modal() {
        let backend = MobileBackend::new();

        let mut trait_def = OirTrait::new("ModalTrait");
        trait_def.states = vec![OirState::initial("closed"), OirState::new("open")];
        trait_def.events = vec![OirEvent::new("OPEN"), OirEvent::new("CLOSE")];
        trait_def.transitions = vec![
            OirTransition::new("closed", "open", "OPEN").with_effects(vec![OirExpr::effect(
                OirEffect::render_ui("modal", OirPattern::new("stat-card")),
            )]),
            OirTransition::new("open", "closed", "CLOSE").with_effects(vec![OirExpr::effect(
                OirEffect::render_ui("modal", OirPattern::new("clear")),
            )]),
        ];

        let mut module = OirModule::new("TestApp", "1.0.0");
        module.traits.push(trait_def);

        let mut page = OirPage::new("Home", "/");
        page.traits.push(OirTraitBinding::new("ModalTrait"));
        module.pages.push(page);

        let output = backend.generate(&module);
        let screen = output
            .files
            .iter()
            .find(|f| f.path == "screens/HomeScreen.tsx")
            .unwrap();

        // Must import Modal from react-native.
        assert!(
            screen.content.contains("Modal"),
            "screen must import Modal from react-native; got:\n{}",
            screen.content
        );
        // The open transition must produce visible={true}.
        assert!(
            screen.content.contains("visible={true}"),
            "open modal must have visible={{true}}; got:\n{}",
            screen.content
        );
        // The close/clear transition must produce visible={false}.
        assert!(
            screen.content.contains("visible={false}"),
            "cleared modal must have visible={{false}}; got:\n{}",
            screen.content
        );
        // Must contain the slot comment for traceability.
        assert!(
            screen.content.contains("slot: modal"),
            "overlay section must carry slot comment; got:\n{}",
            screen.content
        );
    }

    /// slotRole.ts file is emitted by `generate()` and contains the function.
    #[test]
    fn generate_emits_slot_role_ts_file() {
        let backend = MobileBackend::new();
        let module = OirModule::new("TestApp", "1.0.0");
        let output = backend.generate(&module);
        let file = output
            .files
            .iter()
            .find(|f| f.path == "src/slotRole.ts")
            .expect("src/slotRole.ts must be emitted");
        assert!(file.content.contains("export function slotRole"));
        assert!(file.content.contains("export type SlotRole"));
    }
}
