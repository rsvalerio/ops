//! Configuration merge logic: overlay application and field-level merging.

use indexmap::IndexMap;

use super::{Config, ConfigOverlay, OutputConfig, OutputConfigOverlay};

pub(super) fn merge_field<T>(base: &mut T, overlay: Option<T>) {
    if let Some(v) = overlay {
        *base = v;
    }
}

pub(super) fn merge_indexmap<K: Clone + Eq + std::hash::Hash, V: Clone>(
    base: &mut IndexMap<K, V>,
    overlay: Option<&IndexMap<K, V>>,
) {
    if let Some(items) = overlay {
        for (k, v) in items {
            base.insert(k.clone(), v.clone());
        }
    }
}

fn merge_output(base: &mut OutputConfig, overlay: &OutputConfigOverlay) {
    merge_field(&mut base.theme, overlay.theme.clone());
    merge_field(&mut base.columns, overlay.columns);
    merge_field(&mut base.show_error_detail, overlay.show_error_detail);
    merge_field(&mut base.stderr_tail_lines, overlay.stderr_tail_lines);
    merge_field(&mut base.category_order, overlay.category_order.clone());
}

/// Merge overlay into base — only explicitly-set values overwrite.
///
/// Uses destructuring so adding a field to the overlay types without
/// handling it here causes a compile error.
pub fn merge_config(base: &mut Config, overlay: &ConfigOverlay) {
    let ConfigOverlay {
        output,
        commands,
        data,
        themes,
        extensions,
        about,
        stack,
        tools,
    } = overlay;

    if let Some(output_overlay) = output {
        merge_output(&mut base.output, output_overlay);
    }
    merge_indexmap(&mut base.commands, commands.as_ref());
    if let Some(data_overlay) = data {
        if let Some(path) = &data_overlay.path {
            base.data.path = Some(path.clone());
        }
    }
    merge_indexmap(&mut base.themes, themes.as_ref());
    if let Some(ext_overlay) = extensions {
        if let Some(enabled) = &ext_overlay.enabled {
            base.extensions.enabled = Some(enabled.clone());
        }
    }
    if let Some(about_overlay) = about {
        if let Some(fields) = &about_overlay.fields {
            base.about.fields = Some(fields.clone());
        }
    }
    if let Some(s) = stack {
        base.stack = Some(s.clone());
    }
    merge_indexmap(&mut base.tools, tools.as_ref());
}
