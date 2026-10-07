//! Read caret geometry through AT-SPI without reading the field's text.
use futures_util::{stream, StreamExt};
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use zbus::zvariant::OwnedObjectPath;
static CARET_AVAILABLE: AtomicU8 = AtomicU8::new(0);
pub fn availability() -> Option<bool> {
    match CARET_AVAILABLE.load(Ordering::Relaxed) {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CaretKind {
    Caret,
    Field,
    Area,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CaretRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub kind: CaretKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CaretMethod {
    AtSpi,
    Kwin,
    Uia,
    Win32,
    Fallback,
}
pub fn get_caret_position(_target: Option<isize>) -> (Option<CaretRect>, CaretMethod, String) {
    if crate::settings::load_settings().overlay_skin != "mini" {
        CARET_AVAILABLE.store(0, Ordering::Relaxed);
        return (
            None,
            CaretMethod::Fallback,
            "Linux: fixed overlay anchor".into(),
        );
    }
    let result = tauri::async_runtime::block_on(async {
        tokio::time::timeout(Duration::from_millis(1000), query_caret()).await
    });
    let (caret, method, detail) = match result {
        Ok(Ok(Some((rect, method)))) => (
            Some(rect),
            method,
            format!("Linux: {method:?} caret: {rect:?}"),
        ),
        Ok(Ok(None)) => (
            None,
            CaretMethod::Fallback,
            "Linux: active application did not expose visible text caret geometry through AT-SPI"
                .to_string(),
        ),
        Ok(Err(error)) => (
            None,
            CaretMethod::Fallback,
            format!("Linux: AT-SPI query failed: {error}"),
        ),
        Err(_) => (
            None,
            CaretMethod::Fallback,
            "Linux: AT-SPI caret search exceeded 1000 ms".to_string(),
        ),
    };
    CARET_AVAILABLE.store(if caret.is_some() { 1 } else { 2 }, Ordering::Relaxed);
    crate::log_debug(&format!("[linux] {detail}"));
    (caret, method, detail)
}

async fn query_caret() -> zbus::Result<Option<(CaretRect, CaretMethod)>> {
    let active_window = crate::linux::kwin::active_window().await;
    if let Some(window) = active_window {
        crate::log_debug(&format!("[caret] KDE active client: {window:?}"));
        if let Some(rect) = window.text_caret() {
            return Ok(Some((rect, CaretMethod::Kwin)));
        }
    }
    let session = zbus::Connection::session().await?;
    let bus = zbus::Proxy::new(&session, "org.a11y.Bus", "/org/a11y/bus", "org.a11y.Bus").await?;
    let address: String = bus.call("GetAddress", &()).await?;
    let connection = zbus::connection::Builder::address(address.as_str())?
        .build()
        .await?;
    let registry = zbus::Proxy::new(
        &connection,
        "org.a11y.atspi.Registry",
        "/org/a11y/atspi/accessible/root",
        "org.a11y.atspi.Accessible",
    )
    .await?;
    let applications: Vec<(String, OwnedObjectPath)> = registry.call("GetChildren", &()).await?;
    let bus = zbus::Proxy::new(
        &connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .await?;
    let mut searches = stream::iter(applications)
        .map(|root| async {
            let mut client_window = None;
            if let Some(window) = active_window {
                let pid: u32 = bus
                    .call("GetConnectionUnixProcessID", &(&root.0,))
                    .await
                    .unwrap_or(0);
                if pid == window.pid {
                    client_window = Some(window);
                }
            }
            // Do not lose a valid caret because a different application never
            // finishes its accessibility traversal before the outer deadline.
            tokio::time::timeout(
                Duration::from_millis(500),
                find_caret(&connection, root, client_window),
            )
            .await
            .ok()
            .flatten()
        })
        .buffer_unordered(64);
    let mut active = Vec::new();
    let mut focused = Vec::new();
    while let Some(candidate) = searches.next().await {
        if let Some((rect, is_active)) = candidate {
            // An accessibility bridge can have a different PID from the client.
            // In that case accept only an ACTIVE tree with valid screen geometry
            // inside the actual active client; never translate its local origin.
            if active_window.is_some_and(|window| !screen_caret_in_window(rect, window)) {
                continue;
            }
            if is_active {
                active.push(rect);
            } else if active_window.is_none() {
                focused.push(rect);
            }
        }
    }
    // Some Wayland toolkits omit ACTIVE on their toplevel. Accept a unique
    // visible focused text input then, but never race background apps for it.
    Ok(unique_caret(&active)
        .or_else(|| {
            if active.is_empty() {
                unique_caret(&focused)
            } else {
                None
            }
        })
        .map(|rect| (rect, CaretMethod::AtSpi)))
}

async fn find_caret(
    connection: &zbus::Connection,
    root: (String, OwnedObjectPath),
    window: Option<crate::linux::kwin::ActiveWindow>,
) -> Option<(CaretRect, bool)> {
    let mut queue = VecDeque::from([(root.0, root.1, window.is_some())]);
    let mut visited = HashSet::new();
    while !queue.is_empty() && visited.len() < 4096 {
        // A Chromium accessibility tree can contain hundreds of nodes. Inspect
        // siblings concurrently so the hotkey deadline does not expire before
        // reaching the focused input, and start every registered application.
        let mut batch = Vec::new();
        while batch.len() < 16 {
            let Some(node) = queue.pop_front() else { break };
            if visited.insert((node.0.clone(), node.1.clone())) {
                batch.push(node);
            }
        }
        let mut nodes = stream::iter(batch)
            .map(|node| async move {
                // A stale or unresponsive application must not hold up the
                // next level of the tree for the entire hotkey deadline.
                tokio::time::timeout(
                    Duration::from_millis(150),
                    inspect_node(connection, node, window),
                )
                .await
                .unwrap_or((None, Vec::new(), false))
            })
            .buffer_unordered(16);
        while let Some((caret, children, active)) = nodes.next().await {
            if caret.is_some() {
                return caret.map(|rect| (rect, active));
            }
            if active {
                for child in children.into_iter().rev() {
                    queue.push_front((child.0, child.1, active));
                }
            } else {
                queue.extend(children.into_iter().map(|child| (child.0, child.1, active)));
            }
        }
    }
    None
}

async fn inspect_node(
    connection: &zbus::Connection,
    (destination, path, inherited_active): (String, OwnedObjectPath, bool),
    window: Option<crate::linux::kwin::ActiveWindow>,
) -> (Option<CaretRect>, Vec<(String, OwnedObjectPath)>, bool) {
    let Ok(accessible) = zbus::Proxy::new(
        connection,
        destination.as_str(),
        path.as_str(),
        "org.a11y.atspi.Accessible",
    )
    .await
    else {
        return (None, Vec::new(), false);
    };
    let states: Vec<u32> = accessible.call("GetState", &()).await.unwrap_or_default();
    let state = states.first().copied().unwrap_or_default();
    let role: u32 = accessible.call("GetRole", &()).await.unwrap_or_default();
    // ACTIVE is optional in some Wayland toolkits. Hidden toplevels are
    // excluded; visible active toplevels are preferred over ambiguous focus.
    let active = inherited_active || (matches!(role, 16 | 23 | 69) && state & ACTIVE != 0);
    if matches!(role, 16 | 23 | 69) && !visible(state) {
        return (None, Vec::new(), false);
    }
    if state & FOCUSED != 0 {
        crate::log_debug(&format!("[caret] focused node: destination={destination}, role={role}, state={state:#x}, visible={}", visible(state)));
    }
    if visible_editable_caret(state) {
        // Wayland clients may report local coordinates as SCREEN coordinates.
        // With KWin, request WINDOW coordinates and add the actual client origin.
        let coordinate_type = if window.is_some() { 1_u32 } else { 0_u32 };
        let to_screen = |rect| window.map_or(Some(rect), |window| window.screen_caret(rect));
        if let Ok(text) = zbus::Proxy::new(
            connection,
            destination.as_str(),
            path.as_str(),
            "org.a11y.atspi.Text",
        )
        .await
        {
            let offset_result = text.get_property::<i32>("CaretOffset").await;
            if let Err(error) = &offset_result {
                crate::log_debug(&format!(
                    "[caret] focused node has no readable Text.CaretOffset: {error}"
                ));
            }
            if let Ok(offset) = offset_result {
                let count = text
                    .get_property::<i32>("CharacterCount")
                    .await
                    .unwrap_or(-1);
                // A zero-length range can expose the insertion rectangle even
                // in an empty input or on a new line, without reading any text.
                if offset >= 0 && offset <= count {
                    if let Ok(bounds) = text
                        .call::<_, _, (i32, i32, i32, i32)>(
                            "GetRangeExtents",
                            &(offset, offset, coordinate_type),
                        )
                        .await
                    {
                        if bounds.2 == 0 {
                            if let Some(rect) = character_caret(bounds, false).and_then(to_screen) {
                                return (Some(rect), Vec::new(), active);
                            }
                        }
                    }
                }
                // At end-of-text, querying offset == count can return unrelated
                // geometry in some toolkits. Only query actual character indices.
                for (index, after) in character_indices(offset, count) {
                    if index < 0 {
                        continue;
                    }
                    if let Ok(bounds) = text
                        .call::<_, _, (i32, i32, i32, i32)>(
                            "GetCharacterExtents",
                            &(index, coordinate_type),
                        )
                        .await
                    {
                        if let Some(rect) = character_caret(bounds, after).and_then(to_screen) {
                            return (Some(rect), Vec::new(), active);
                        }
                    }
                }
            }
        }
    }
    let children: Vec<(String, OwnedObjectPath)> = accessible
        .call("GetChildren", &())
        .await
        .unwrap_or_default();
    (None, children, active)
}

const ACTIVE: u32 = 1 << 1;
const FOCUSED: u32 = 1 << 12;
const SHOWING: u32 = 1 << 25;
const VISIBLE: u32 = 1 << 30;

fn visible(state: u32) -> bool {
    state & (SHOWING | VISIBLE) == SHOWING | VISIBLE
}

fn visible_editable_caret(state: u32) -> bool {
    // Text/CaretOffset and valid extents establish this is a text insertion
    // point. Requiring EDITABLE here excludes toolkits that omit that flag.
    state & FOCUSED != 0 && visible(state)
}

fn unique_caret(rects: &[CaretRect]) -> Option<CaretRect> {
    match rects {
        [rect] => Some(*rect),
        _ => None,
    }
}

fn screen_caret_in_window(rect: CaretRect, window: crate::linux::kwin::ActiveWindow) -> bool {
    rect.left as f64 >= window.x
        && rect.top as f64 >= window.y
        && (rect.right as f64) <= window.x + window.width
        && (rect.bottom as f64) <= window.y + window.height
}

fn character_indices(offset: i32, count: i32) -> Vec<(i32, bool)> {
    if offset < 0 || count < 0 || offset > count {
        return Vec::new();
    }
    let mut indices = Vec::new();
    if offset < count {
        indices.push((offset, false));
    }
    if offset > 0 {
        indices.push((offset - 1, true));
    }
    indices
}

fn character_caret((x, y, width, height): (i32, i32, i32, i32), after: bool) -> Option<CaretRect> {
    if height <= 0 || x <= -100000 || y <= -100000 {
        return None;
    }
    if width < 0 {
        return None;
    }
    let left = if after {
        x.checked_add(width.max(0))?
    } else {
        x
    };
    Some(CaretRect {
        left,
        top: y,
        right: left.checked_add(1)?,
        bottom: y.checked_add(height)?,
        kind: CaretKind::Caret,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_plasma_focus_is_rejected_but_wayland_flags_are_optional() {
        assert!(!visible_editable_caret(16783744));
        assert!(visible(SHOWING | VISIBLE));
        assert!(visible_editable_caret(FOCUSED | SHOWING | VISIBLE));
        assert!(!visible_editable_caret(FOCUSED | VISIBLE));
    }

    #[test]
    fn ambiguous_focused_inputs_are_not_chosen_by_response_order() {
        let rect = character_caret((100, 200, 4, 18), false).unwrap();
        assert_eq!(unique_caret(&[rect]), Some(rect));
        assert_eq!(unique_caret(&[rect, rect]), None);
        assert_eq!(unique_caret(&[]), None);
    }

    #[test]
    fn bridge_screen_coordinates_must_belong_to_active_client() {
        let window = crate::linux::kwin::ActiveWindow {
            pid: 1,
            x: 100.0,
            y: 200.0,
            width: 600.0,
            height: 400.0,
            caret: None,
        };
        let rect = character_caret((150, 250, 0, 18), false).unwrap();
        assert!(screen_caret_in_window(rect, window));
        assert!(!screen_caret_in_window(
            CaretRect { left: 50, ..rect },
            window
        ));
        assert!(!screen_caret_in_window(
            CaretRect {
                bottom: 601,
                ..rect
            },
            window
        ));
    }

    #[test]
    fn end_of_text_does_not_query_out_of_range_offset() {
        assert_eq!(character_indices(3, 3), vec![(2, true)]);
        assert_eq!(character_indices(1, 3), vec![(1, false), (0, true)]);
        assert!(character_indices(0, 0).is_empty());
        assert!(character_indices(4, 3).is_empty());
        assert!(character_indices(-1, 3).is_empty());
    }
    #[test]
    fn end_of_text_uses_previous_character_right_edge() {
        assert_eq!(
            character_caret((40, 50, 9, 18), true).map(|r| (r.left, r.bottom)),
            Some((49, 68))
        );
        assert_eq!(
            character_caret((40, 50, 9, 18), false).map(|r| r.left),
            Some(40)
        );
        assert!(character_caret((0, 0, 0, 0), false).is_none());
        assert!(character_caret((i32::MIN, i32::MIN, 0, 18), false).is_none());
    }
}
