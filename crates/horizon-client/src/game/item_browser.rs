//! The item and recipe browser extra (`item_browser`, `docs/extras.md`), off
//! by default: with it on, the inventory and container screens get a panel
//! of every item on their right (the creative search tab's stacks and its
//! search, `@namespace` too), and R or U over an item, in the panel, the
//! inventory or a recipe, shows the recipes that make it or use it. A left
//! click in the panel shows its recipes, a right click its uses; a creative
//! player in single player takes the stack instead (Shift: a full one).
use super::{Game, Screen};
use horizon_client::{
    hud::{HudBatch, rgba},
    item_browser::{self, Lookup, Shown},
    items::ItemStack,
    ui::EditBox,
    widgets,
};
use std::time::Instant;
use winit::keyboard::KeyCode;

/// A slot's side in GUI pixels.
const SLOT: f32 = 18.0;
/// The recipe frame's size.
const FRAME_W: f32 = 176.0;
const FRAME_H: f32 = 112.0;

pub(super) struct ItemBrowser {
    search: EditBox,
    page: usize,
    view: Option<View>,
    /// The views R and U replaced, for Backspace.
    history: Vec<View>,
    lookup: Option<Lookup>,
    /// The last query and its stacks (indices into the creative search tab).
    results: Option<(String, Vec<usize>)>,
    started: Instant,
    /// The recipe frame's slots drawn last, and its arrows and rectangle.
    slots: Vec<([f32; 4], String)>,
    arrows: [[f32; 4]; 2],
    frame: [f32; 4],
}
impl Default for ItemBrowser {
    fn default() -> Self {
        let mut search = EditBox::new(0.0, 0.0, 100.0, 14.0);
        search.focused = false;
        search.max_length = 50;
        Self {
            search,
            page: 0,
            view: None,
            history: Vec::new(),
            lookup: None,
            results: None,
            started: Instant::now(),
            slots: Vec::new(),
            arrows: [[0.0; 4]; 2],
            frame: [0.0; 4],
        }
    }
}
/// The recipes shown for one item.
#[derive(Clone, Debug)]
struct View {
    item: String,
    uses: bool,
    list: Vec<usize>,
    index: usize,
}
/// Where the panel is: its grid, search box and page arrows.
#[derive(Clone, Copy, Debug)]
struct Layout {
    x: f32,
    y: f32,
    cols: usize,
    rows: usize,
    search: [f32; 4],
    prev: [f32; 4],
    next: [f32; 4],
    page_y: f32,
}
fn inside([x, y, w, h]: [f32; 4], (cx, cy): (f32, f32)) -> bool {
    cx >= x && cx < x + w && cy >= y && cy < y + h
}

impl Game<'_> {
    /// The open screen gets the browser: the inventory or a container, not
    /// the stations with their own screens.
    fn item_browser_on(&self) -> bool {
        self.settings.extras.recipe_browser
            && self.screen == Screen::Inventory
            && self.station.is_none()
            && !self.mount_active()
    }
    fn item_browser_layout(&self, batch: &HudBatch) -> Option<Layout> {
        let (ox, _) = self.inventory_origin(batch);
        let left = ox + 176.0 + 8.0;
        let right = batch.width - 6.0;
        let cols = (((right - left) / SLOT).floor().max(0.0) as usize).min(16);
        let top = 6.0;
        let grid_y = top + 20.0;
        let page_y = batch.height - 16.0;
        let rows = ((page_y - 4.0 - grid_y) / SLOT).floor().max(0.0) as usize;
        if cols < 3 || rows < 2 {
            return None;
        }
        let w = cols as f32 * SLOT;
        Some(Layout {
            x: left,
            y: grid_y,
            cols,
            rows,
            search: [left, top, w, 14.0],
            prev: [left, page_y, 12.0, 12.0],
            next: [left + w - 12.0, page_y, 12.0, 12.0],
            page_y,
        })
    }
    /// The stacks the query keeps, in the creative search tab's order.
    fn item_browser_results(&mut self) -> Vec<usize> {
        let query = self.item_browser.search.value.clone();
        if let Some((q, r)) = &self.item_browser.results
            && *q == query
        {
            return r.clone();
        }
        let (namespace, term) = item_browser::split_query(&query);
        let stacks = self.creative_screen.search_items();
        let found: Vec<usize> = self
            .creative_screen
            .search_results(&term, &self.pack.names)
            .into_iter()
            .filter(|&i| item_browser::in_namespace(&stacks[i].item, namespace.as_deref()))
            .collect();
        self.item_browser.results = Some((query, found.clone()));
        found
    }
    /// The panel's stack under the cursor.
    fn item_browser_hovered(&mut self, batch: &HudBatch) -> Option<ItemStack> {
        let layout = self.item_browser_layout(batch)?;
        let (cx, cy) = self.cursor;
        if cx < layout.x || cy < layout.y {
            return None;
        }
        let (col, row) = (
            ((cx - layout.x) / SLOT) as usize,
            ((cy - layout.y) / SLOT) as usize,
        );
        if col >= layout.cols || row >= layout.rows {
            return None;
        }
        let per_page = layout.cols * layout.rows;
        let results = self.item_browser_results();
        let i = results.get(self.item_browser.page * per_page + row * layout.cols + col)?;
        self.creative_screen.search_items().get(*i).cloned()
    }
    /// The item under the cursor: in the recipe frame, the panel or a slot.
    fn item_browser_target(&mut self) -> Option<String> {
        let batch = self.item_browser_batch()?;
        if self.item_browser.view.is_some() {
            return self
                .item_browser
                .slots
                .iter()
                .find(|(r, _)| inside(*r, self.cursor))
                .map(|(_, id)| id.clone());
        }
        if let Some(stack) = self.item_browser_hovered(&batch) {
            return Some(stack.item.to_string());
        }
        let slot = self.slot_at(&batch, self.cursor.0, self.cursor.1)?;
        self.slot_view(slot).map(|s| s.item.to_string())
    }
    fn item_browser_batch(&self) -> Option<HudBatch> {
        let r = self.renderer.as_ref()?;
        Some(HudBatch::new([r.config.width, r.config.height]))
    }
    /// Shows the recipes making (or using) `item`; false when there are none.
    fn item_browser_show(&mut self, item: String, uses: bool) -> bool {
        if self.item_browser.lookup.is_none() {
            self.item_browser.lookup = Some(Lookup::collect(
                self.res.recipes(),
                &self.res.stations().recipes,
            ));
        }
        let lookup = self.item_browser.lookup.as_ref().expect("built above");
        let list = if uses {
            lookup.using(&item)
        } else {
            lookup.making(&item)
        };
        if list.is_empty() {
            return false;
        }
        let view = View {
            item,
            uses,
            list: list.to_vec(),
            index: 0,
        };
        if let Some(old) = self.item_browser.view.replace(view) {
            self.item_browser.history.push(old);
        }
        self.item_browser.search.focused = false;
        true
    }
    fn item_browser_step(&mut self, forward: bool) {
        if let Some(v) = &mut self.item_browser.view {
            let n = v.list.len();
            v.index = if forward {
                (v.index + 1) % n
            } else {
                (v.index + n - 1) % n
            };
        }
    }
    /// The screen closed: back to the panel, the query kept.
    pub(super) fn item_browser_close(&mut self) {
        self.item_browser.view = None;
        self.item_browser.history.clear();
        self.item_browser.search.focused = false;
    }

    // ---------------------------------------------------------------- input

    /// Whether typed keys go to the search box.
    pub(super) fn item_browser_typing(&self) -> bool {
        self.item_browser_on() && self.item_browser.search.focused
    }
    /// A key press while the search box has focus (`EditBox.keyPressed`).
    pub(super) fn item_browser_type(&mut self, event: &winit::event::KeyEvent) {
        use winit::keyboard::PhysicalKey;
        if let Some(shortcut) = self.edit_shortcut(event) {
            self.item_browser
                .search
                .shortcut(shortcut, &mut self.clipboard);
            self.item_browser.page = 0;
            return;
        }
        let ctrl = self.has_control_down();
        let shift = self.has_shift_down();
        let search = &mut self.item_browser.search;
        let handled = match event.physical_key {
            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                search.focused = false;
                true
            }
            PhysicalKey::Code(code) => search.key(code, ctrl, shift),
            _ => false,
        };
        if !handled
            && !ctrl
            && let Some(text) = &event.text
        {
            let text: String = text.chars().filter(|c| !c.is_control()).collect();
            search.insert_text(&text);
        }
        search.reset_blink();
        self.item_browser.page = 0;
    }
    /// Text typed through the control socket (`horizon-ctl type`).
    pub(super) fn item_browser_text(&mut self, text: Option<&str>, backspace: bool) {
        let search = &mut self.item_browser.search;
        if backspace {
            search.delete_chars(-1);
        } else if let Some(text) = text {
            search.insert_text(text);
        }
        self.item_browser.page = 0;
    }
    /// R, U, the arrows, Backspace and Escape for the browser (true: taken).
    pub(super) fn item_browser_key(&mut self, key: KeyCode) -> bool {
        if !self.item_browser_on() {
            return false;
        }
        if self.item_browser.search.focused {
            if key == KeyCode::Escape {
                self.item_browser.search.focused = false;
                return true;
            }
            return false;
        }
        if self.recipe_search_focused() {
            return false;
        }
        match key {
            KeyCode::KeyR | KeyCode::KeyU => {
                let Some(item) = self.item_browser_target() else {
                    return false;
                };
                self.item_browser_show(item, key == KeyCode::KeyU);
                true
            }
            KeyCode::Escape | KeyCode::Backspace if self.item_browser.view.is_some() => {
                self.item_browser.view = self.item_browser.history.pop();
                true
            }
            KeyCode::ArrowLeft | KeyCode::ArrowRight if self.item_browser.view.is_some() => {
                self.item_browser_step(key == KeyCode::ArrowRight);
                true
            }
            KeyCode::KeyE if self.item_browser.view.is_some() => {
                // The inventory key closes the frame first.
                self.item_browser_close();
                true
            }
            _ => false,
        }
    }
    /// A mouse press on the browser (true: taken, the screen does not see it).
    pub(super) fn item_browser_press(&mut self, right: bool) -> bool {
        if !self.item_browser_on() {
            return false;
        }
        let Some(batch) = self.item_browser_batch() else {
            return false;
        };
        let cursor = self.cursor;
        if self.item_browser.view.is_some() {
            if !inside(self.item_browser.frame, cursor) {
                self.item_browser_close();
                return true;
            }
            if inside(self.item_browser.arrows[0], cursor) {
                self.item_browser_step(false);
            } else if inside(self.item_browser.arrows[1], cursor) {
                self.item_browser_step(true);
            } else if let Some((_, id)) = self
                .item_browser
                .slots
                .iter()
                .find(|(r, _)| inside(*r, cursor))
                .cloned()
            {
                self.item_browser_show(id, right);
            }
            return true;
        }
        let Some(layout) = self.item_browser_layout(&batch) else {
            return false;
        };
        self.item_browser.search.focused = inside(layout.search, cursor);
        if self.item_browser.search.focused {
            if right {
                self.item_browser.search.set_value("");
                self.item_browser.page = 0;
            } else {
                let shift = self.has_shift_down();
                self.item_browser
                    .search
                    .click(&self.gui.atlas, cursor.0, shift, false);
            }
            self.item_browser.search.reset_blink();
            return true;
        }
        let per_page = layout.cols * layout.rows;
        let pages = item_browser::pages(self.item_browser_results().len(), per_page);
        if inside(layout.prev, cursor) {
            self.item_browser.page = (self.item_browser.page + pages - 1) % pages;
            return true;
        }
        if inside(layout.next, cursor) {
            self.item_browser.page = (self.item_browser.page + 1) % pages;
            return true;
        }
        let in_panel = inside(
            [
                layout.x,
                layout.y,
                layout.cols as f32 * SLOT,
                layout.rows as f32 * SLOT,
            ],
            cursor,
        );
        if !in_panel {
            return false;
        }
        if self.creative && self.remote.is_none() {
            // As the creative tab: the stack goes to the player. Not to the
            // cursor: a click ends with a release, and the release would drop
            // what the press picked up (in creative, into nothing).
            if let Some(mut stack) = self.item_browser_hovered(&batch) {
                stack.count = if right && !self.has_shift_down() {
                    1
                } else {
                    self.max_stack(&stack)
                };
                if let Some(rest) = self.give(stack) {
                    self.player_drop(rest);
                }
            }
            return true;
        }
        if let Some(stack) = self.item_browser_hovered(&batch) {
            self.item_browser_show(stack.item.to_string(), right);
        }
        true
    }
    /// The wheel over the panel turns its pages (true: taken).
    pub(super) fn item_browser_scroll(&mut self, amount: f32) -> bool {
        if !self.item_browser_on() || self.item_browser.view.is_some() {
            return false;
        }
        let Some(batch) = self.item_browser_batch() else {
            return false;
        };
        let Some(layout) = self.item_browser_layout(&batch) else {
            return false;
        };
        let area = [
            layout.x,
            layout.search[1],
            layout.cols as f32 * SLOT,
            layout.page_y + 12.0 - layout.search[1],
        ];
        if !inside(area, self.cursor) {
            return false;
        }
        let pages =
            item_browser::pages(self.item_browser_results().len(), layout.cols * layout.rows);
        self.item_browser.page = if amount < 0.0 {
            (self.item_browser.page + 1) % pages
        } else {
            (self.item_browser.page + pages - 1) % pages
        };
        true
    }

    // ---------------------------------------------------------------- drawing

    /// The panel, its hovered item's tooltip and the recipe frame, over the screen.
    pub(super) fn draw_item_browser(&mut self, batch: &mut HudBatch) {
        if !self.item_browser_on() {
            return;
        }
        if let Some(layout) = self.item_browser_layout(batch) {
            self.draw_item_browser_panel(batch, layout);
        }
        if self.item_browser.view.is_some() {
            self.draw_item_browser_frame(batch);
        } else if self.carried.is_none()
            && let Some(stack) = self.item_browser_hovered(batch)
        {
            let mut lines = self.stack_tooltip(&stack);
            lines.push(("R: recipes  U: uses".into(), rgba(128, 128, 128, 255)));
            self.draw_tooltip_lines(batch, &lines, self.cursor.0, self.cursor.1);
        }
    }
    fn draw_item_browser_panel(&mut self, batch: &mut HudBatch, layout: Layout) {
        let [sx, sy, sw, sh] = layout.search;
        // A dark panel under the search box and the grid, as JEI's: the items
        // read against it and the HUD behind does not show between them.
        batch.fill(
            &self.gui.atlas,
            sx - 3.0,
            sy - 3.0,
            sw + 6.0,
            (layout.page_y + 12.0 - sy) + 6.0,
            rgba(16, 16, 16, 190),
        );
        {
            let search = &mut self.item_browser.search;
            search.x = sx;
            search.y = sy;
            search.width = sw;
            search.height = sh;
            search.draw(batch, &self.gui.atlas, None);
            if search.value.is_empty() && !search.focused {
                batch.text(
                    &self.gui.atlas,
                    "Search items",
                    sx + 4.0,
                    sy + 3.0,
                    rgba(128, 128, 128, 255),
                    false,
                );
            }
        }
        let results = self.item_browser_results();
        let per_page = layout.cols * layout.rows;
        let pages = item_browser::pages(results.len(), per_page);
        self.item_browser.page = self.item_browser.page.min(pages - 1);
        let first = self.item_browser.page * per_page;
        let shown: Vec<ItemStack> = results
            .iter()
            .skip(first)
            .take(per_page)
            .filter_map(|&i| self.creative_screen.search_items().get(i).cloned())
            .collect();
        for (n, stack) in shown.iter().enumerate() {
            let x = layout.x + (n % layout.cols) as f32 * SLOT + 1.0;
            let y = layout.y + (n / layout.cols) as f32 * SLOT + 1.0;
            batch.fill(
                &self.gui.atlas,
                x - 1.0,
                y - 1.0,
                SLOT,
                SLOT,
                rgba(90, 90, 90, 70),
            );
            if inside([x - 1.0, y - 1.0, SLOT, SLOT], self.cursor) {
                batch.fill(&self.gui.atlas, x, y, 16.0, 16.0, rgba(255, 255, 255, 80));
            }
            self.draw_item(batch, stack, x, y);
        }
        let atlas = &self.gui.atlas;
        for (rect, label, text) in [
            (layout.prev, "Previous items page", "<"),
            (layout.next, "Next items page", ">"),
        ] {
            draw_button(batch, atlas, rect, text, inside(rect, self.cursor));
            widgets::record(label, rect, pages > 1);
        }
        let page = format!("{}/{}", self.item_browser.page + 1, pages);
        let w = HudBatch::text_width(atlas, &page);
        let mid = layout.x + layout.cols as f32 * SLOT / 2.0;
        batch.text(
            atlas,
            &page,
            mid - w / 2.0,
            layout.page_y + 2.0,
            u32::MAX,
            true,
        );
    }
    fn draw_item_browser_frame(&mut self, batch: &mut HudBatch) {
        let Some(view) = self.item_browser.view.clone() else {
            return;
        };
        let Some(shown) = self
            .item_browser
            .lookup
            .as_ref()
            .and_then(|l| l.get(view.list[view.index]))
            .cloned()
        else {
            return;
        };
        let millis = self.item_browser.started.elapsed().as_millis();
        let (bx, by) = batch.centered_origin(FRAME_W, FRAME_H);
        let frame = [bx, by, FRAME_W, FRAME_H];
        self.item_browser.frame = frame;
        let mut slots: Vec<([f32; 4], String, u32)> = Vec::new();
        {
            let atlas = &self.gui.atlas;
            batch.fill(
                atlas,
                0.0,
                0.0,
                batch.width,
                batch.height,
                rgba(16, 16, 16, 120),
            );
            batch.fill(atlas, bx, by, FRAME_W, FRAME_H, rgba(30, 30, 30, 255));
            batch.fill(
                atlas,
                bx + 1.0,
                by + 1.0,
                FRAME_W - 2.0,
                FRAME_H - 2.0,
                rgba(52, 52, 52, 255),
            );
            let name = self.pack.names.item(&view.item);
            let heading = if view.uses {
                format!("Uses of {name}")
            } else {
                format!("Recipes for {name}")
            };
            batch.text(
                atlas,
                &heading,
                bx + 8.0,
                by + 6.0,
                rgba(238, 238, 238, 255),
                false,
            );
            batch.text(
                atlas,
                shown.title(),
                bx + 26.0,
                by + 22.0,
                rgba(238, 238, 238, 255),
                false,
            );
        }
        let station = ItemStack::new(shown.station(), 1);
        self.draw_item(batch, &station, bx + 8.0, by + 18.0);
        let grid = (bx + 20.0, by + 40.0);
        let slot = |x: f32, y: f32, items: &[String], count: u32, out: &mut Vec<_>| {
            if let Some(id) = item_browser::cycled(items, millis) {
                out.push(([x, y, SLOT, SLOT], id.to_owned(), count));
            }
        };
        let result_x;
        let arrow_x;
        match &shown {
            Shown::Crafting {
                width,
                height,
                cells,
                result,
                count,
                ..
            } => {
                for (i, cell) in cells.iter().enumerate() {
                    let (cx, cy) = (i % width, i / width);
                    slot(
                        grid.0 + cx as f32 * SLOT,
                        grid.1 + cy as f32 * SLOT,
                        cell,
                        1,
                        &mut slots,
                    );
                }
                arrow_x = grid.0 + 3.0 * SLOT + 8.0;
                result_x = arrow_x + 26.0;
                slot(
                    result_x,
                    grid.1 + SLOT,
                    std::slice::from_ref(result),
                    *count,
                    &mut slots,
                );
                self.draw_item_browser_backgrounds(batch, grid, *width, *height);
            }
            Shown::Cooking {
                input,
                result,
                count,
                time,
                experience,
                ..
            } => {
                slot(grid.0, grid.1 + SLOT, input, 1, &mut slots);
                arrow_x = grid.0 + SLOT + 10.0;
                result_x = arrow_x + 26.0;
                slot(
                    result_x,
                    grid.1 + SLOT,
                    std::slice::from_ref(result),
                    *count,
                    &mut slots,
                );
                self.draw_item_browser_backgrounds(batch, (grid.0, grid.1 + SLOT), 1, 1);
                let info = format!("{:.1} s, {} XP", *time as f32 / 20.0, experience);
                batch.text(
                    &self.gui.atlas,
                    &info,
                    grid.0,
                    grid.1 + 2.0 * SLOT + 6.0,
                    rgba(238, 238, 238, 255),
                    false,
                );
            }
            Shown::Stonecutting {
                input,
                result,
                count,
            } => {
                slot(grid.0, grid.1 + SLOT, input, 1, &mut slots);
                arrow_x = grid.0 + SLOT + 10.0;
                result_x = arrow_x + 26.0;
                slot(
                    result_x,
                    grid.1 + SLOT,
                    std::slice::from_ref(result),
                    *count,
                    &mut slots,
                );
                self.draw_item_browser_backgrounds(batch, (grid.0, grid.1 + SLOT), 1, 1);
            }
            Shown::Smithing {
                template,
                base,
                addition,
                result,
            } => {
                for (i, items) in [template, base, addition].into_iter().enumerate() {
                    slot(
                        grid.0 + i as f32 * SLOT,
                        grid.1 + SLOT,
                        items,
                        1,
                        &mut slots,
                    );
                }
                arrow_x = grid.0 + 3.0 * SLOT + 8.0;
                result_x = arrow_x + 26.0;
                let made = match result {
                    Some(r) => std::slice::from_ref(r),
                    None => base.as_slice(),
                };
                slot(result_x, grid.1 + SLOT, made, 1, &mut slots);
                self.draw_item_browser_backgrounds(batch, (grid.0, grid.1 + SLOT), 3, 1);
            }
        }
        {
            let atlas = &self.gui.atlas;
            draw_result_arrow(batch, atlas, arrow_x, grid.1 + SLOT + 3.5);
            slot_background(batch, atlas, result_x - 1.0, grid.1 + SLOT - 1.0);
        }
        for (r, id, count) in &slots {
            let stack = ItemStack::new(id, *count);
            self.draw_item(batch, &stack, r[0], r[1]);
        }
        // Arrows and the page of recipes.
        let atlas = &self.gui.atlas;
        let prev = [bx + 8.0, by + FRAME_H - 18.0, 12.0, 12.0];
        let next = [bx + FRAME_W - 20.0, by + FRAME_H - 18.0, 12.0, 12.0];
        self.item_browser.arrows = [prev, next];
        let many = view.list.len() > 1;
        for (rect, label, text) in [(prev, "Previous recipe", "<"), (next, "Next recipe", ">")] {
            draw_button(batch, atlas, rect, text, inside(rect, self.cursor));
            widgets::record(label, rect, many);
        }
        let page = format!("{}/{}", view.index + 1, view.list.len());
        let w = HudBatch::text_width(atlas, &page);
        batch.text(
            atlas,
            &page,
            bx + FRAME_W / 2.0 - w / 2.0,
            by + FRAME_H - 16.0,
            rgba(238, 238, 238, 255),
            false,
        );
        let hovered = slots.iter().find(|(r, _, _)| inside(*r, self.cursor));
        let tooltip = hovered.map(|(_, id, count)| ItemStack::new(id, *count));
        self.item_browser.slots = slots.into_iter().map(|(r, id, _)| (r, id)).collect();
        if let Some(stack) = tooltip {
            let mut lines = self.stack_tooltip(&stack);
            lines.push(("R: recipes  U: uses".into(), rgba(128, 128, 128, 255)));
            self.draw_tooltip_lines(batch, &lines, self.cursor.0, self.cursor.1);
        }
    }
    /// The grid's empty slots, as the crafting table draws them.
    fn draw_item_browser_backgrounds(
        &self,
        batch: &mut HudBatch,
        (x, y): (f32, f32),
        width: usize,
        height: usize,
    ) {
        for row in 0..height {
            for col in 0..width {
                slot_background(
                    batch,
                    &self.gui.atlas,
                    x + col as f32 * SLOT - 1.0,
                    y + row as f32 * SLOT - 1.0,
                );
            }
        }
    }
}

/// The arrow from a recipe's ingredients to its result: a shaft and a
/// stepped head, light on the frame's dark face.
fn draw_result_arrow(batch: &mut HudBatch, atlas: &horizon_client::hud::HudAtlas, x: f32, y: f32) {
    let colour = rgba(210, 210, 210, 255);
    batch.fill(atlas, x, y + 3.0, 10.0, 3.0, colour);
    batch.fill(atlas, x + 10.0, y + 1.0, 2.0, 7.0, colour);
    batch.fill(atlas, x + 12.0, y + 2.0, 2.0, 5.0, colour);
    batch.fill(atlas, x + 14.0, y + 3.0, 2.0, 3.0, colour);
}
/// An inventory slot's sunken square.
fn slot_background(batch: &mut HudBatch, atlas: &horizon_client::hud::HudAtlas, x: f32, y: f32) {
    batch.fill(atlas, x, y, SLOT, SLOT, rgba(55, 55, 55, 255));
    batch.fill(
        atlas,
        x + 1.0,
        y + 1.0,
        17.0,
        17.0,
        rgba(255, 255, 255, 255),
    );
    batch.fill(
        atlas,
        x + 1.0,
        y + 1.0,
        16.0,
        16.0,
        rgba(139, 139, 139, 255),
    );
}
/// A small bordered button with a glyph, lighter under the cursor.
fn draw_button(
    batch: &mut HudBatch,
    atlas: &horizon_client::hud::HudAtlas,
    [x, y, w, h]: [f32; 4],
    text: &str,
    hovered: bool,
) {
    let face = if hovered {
        rgba(220, 220, 220, 255)
    } else {
        rgba(160, 160, 160, 255)
    };
    batch.fill(atlas, x, y, w, h, rgba(55, 55, 55, 255));
    batch.fill(atlas, x + 1.0, y + 1.0, w - 2.0, h - 2.0, face);
    let tw = HudBatch::text_width(atlas, text);
    batch.text(
        atlas,
        text,
        x + (w - tw) / 2.0,
        y + 2.0,
        rgba(55, 55, 55, 255),
        false,
    );
}
