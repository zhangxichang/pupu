use std::{cell::RefCell, rc::Rc};

use slint::{ComponentHandle, Global, Model};

use crate::{GroupListLayout, PrivateListLayout, Viewport, VirtualListSlot};

const ITEM_HEIGHT_ESTIMATE: f32 = 56.0;
const MIN_VALID_ROW_HEIGHT: f32 = 24.0;
const OVERSCAN: f32 = 300.0;

struct Layout {
    heights: Vec<f32>,
    viewport_offset: f32,
    viewport_height: f32,
    content_height: f32,
    slots: Vec<VirtualListSlot>,
    pool_capacity: usize,
    model: Option<Rc<slint::VecModel<VirtualListSlot>>>,
    pushed_content_height: f32,
    pushed_slots: Vec<(i32, f32)>,
}

impl Layout {
    fn new() -> Self {
        Self {
            heights: Vec::new(),
            viewport_offset: 0.0,
            viewport_height: 0.0,
            content_height: 0.0,
            slots: Vec::new(),
            pool_capacity: 0,
            model: None,
            pushed_content_height: -1.0,
            pushed_slots: Vec::new(),
        }
    }

    fn reset(&mut self, count: i32) {
        self.heights = vec![0.0; count.max(0) as usize];
        self.viewport_offset = 0.0;
        self.update();
    }

    fn set_count(&mut self, count: i32) {
        let count = count.max(0) as usize;
        self.heights.resize(count, 0.0);
        self.update();
    }

    fn set_height(&mut self, index: i32, height: f32) {
        let index = index.max(0) as usize;
        let Some(old_height) = self.heights.get(index).copied() else {
            return;
        };
        if height < MIN_VALID_ROW_HEIGHT || (height - old_height).abs() < 0.5 {
            return;
        }
        self.heights[index] = height;
        self.update();
    }

    fn set_viewport(&mut self, offset: f32, height: f32) {
        let offset = offset.max(0.0);
        if (offset - self.viewport_offset).abs() < 0.5
            && (height - self.viewport_height).abs() < 0.5
        {
            return;
        }
        self.viewport_offset = offset;
        self.viewport_height = height.max(0.0);
        self.update();
    }

    fn update(&mut self) {
        let capacity = ((((self.viewport_height + 2.0 * OVERSCAN) / ITEM_HEIGHT_ESTIMATE).ceil()
            as usize)
            .saturating_add(4))
        .clamp(8, 256);
        if capacity > self.pool_capacity {
            self.slots
                .resize_with(capacity, || VirtualListSlot { index: -1, y: 0.0 });
            self.pool_capacity = capacity;
        }

        let mut y = 0.0;
        let mut visible = Vec::new();
        let start = (self.viewport_offset - OVERSCAN).max(0.0);
        let end = self.viewport_offset + self.viewport_height + OVERSCAN;

        for (index, measured) in self.heights.iter().enumerate() {
            let height = if *measured > 0.0 {
                *measured
            } else {
                ITEM_HEIGHT_ESTIMATE
            };
            if y + height >= start && y <= end {
                visible.push(VirtualListSlot {
                    index: index as i32,
                    y,
                });
            }
            y += height;
        }

        self.content_height = y;

        let mut wanted = vec![false; self.heights.len()];
        for slot in &visible {
            if slot.index >= 0 && (slot.index as usize) < wanted.len() {
                wanted[slot.index as usize] = true;
            }
        }
        for slot in &mut self.slots {
            if slot.index >= 0
                && ((slot.index as usize) >= wanted.len() || !wanted[slot.index as usize])
            {
                slot.index = -1;
                slot.y = 0.0;
            }
        }

        let mut positions = vec![usize::MAX; self.heights.len()];
        for (position, slot) in self.slots.iter().enumerate() {
            if slot.index >= 0 && (slot.index as usize) < positions.len() {
                positions[slot.index as usize] = position;
            }
        }

        let mut free = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.index < 0)
            .map(|(position, _)| position)
            .collect::<Vec<_>>()
            .into_iter();

        for desired in &visible {
            if desired.index < 0 || (desired.index as usize) >= positions.len() {
                continue;
            }
            let index = desired.index as usize;
            let position = positions[index];
            if position != usize::MAX {
                self.slots[position].y = desired.y;
            } else if let Some(position) = free.next() {
                self.slots[position] = desired.clone();
                positions[index] = position;
            } else {
                positions[index] = self.slots.len();
                self.slots.push(desired.clone());
                self.pool_capacity = self.slots.len();
            }
        }
    }
}

fn sync_slots(model: &slint::VecModel<VirtualListSlot>, slots: &[VirtualListSlot]) {
    let mut count = model.row_count();
    while count > slots.len() {
        model.remove(count - 1);
        count -= 1;
    }
    for (index, slot) in slots.iter().enumerate() {
        if index < count {
            let changed = match model.row_data(index) {
                Some(current) => current.index != slot.index || (current.y - slot.y).abs() >= 0.01,
                None => true,
            };
            if changed {
                model.set_row_data(index, slot.clone());
            }
        } else {
            let _ = model.push_row(slot.clone());
        }
    }
}

fn push_content(global: &PrivateListLayout, layout: &mut Layout) {
    if layout.model.is_none() {
        let model = Rc::new(slint::VecModel::default());
        global.set_slots(model.clone().into());
        layout.model = Some(model);
    }
    if (layout.content_height - layout.pushed_content_height).abs() >= 0.5 {
        layout.pushed_content_height = layout.content_height;
        global.set_content_height(layout.content_height);
    }
    let changed = layout.pushed_slots.len() != layout.slots.len()
        || layout
            .slots
            .iter()
            .zip(layout.pushed_slots.iter())
            .any(|(slot, pushed)| slot.index != pushed.0 || (slot.y - pushed.1).abs() >= 0.01);
    if changed {
        layout.pushed_slots.clear();
        layout
            .pushed_slots
            .extend(layout.slots.iter().map(|slot| (slot.index, slot.y)));
        let model = layout.model.as_ref().unwrap().clone();
        sync_slots(&model, &layout.slots);
    }
}

fn push_content_group(global: &GroupListLayout, layout: &mut Layout) {
    if layout.model.is_none() {
        let model = Rc::new(slint::VecModel::default());
        global.set_slots(model.clone().into());
        layout.model = Some(model);
    }
    if (layout.content_height - layout.pushed_content_height).abs() >= 0.5 {
        layout.pushed_content_height = layout.content_height;
        global.set_content_height(layout.content_height);
    }
    let changed = layout.pushed_slots.len() != layout.slots.len()
        || layout
            .slots
            .iter()
            .zip(layout.pushed_slots.iter())
            .any(|(slot, pushed)| slot.index != pushed.0 || (slot.y - pushed.1).abs() >= 0.01);
    if changed {
        layout.pushed_slots.clear();
        layout
            .pushed_slots
            .extend(layout.slots.iter().map(|slot| (slot.index, slot.y)));
        let model = layout.model.as_ref().unwrap().clone();
        sync_slots(&model, &layout.slots);
    }
}

pub fn setup(viewport: &Viewport) {
    setup_private(viewport);
    setup_group(viewport);
}

fn setup_private(viewport: &Viewport) {
    let global = PrivateListLayout::get(viewport);
    let state = Rc::new(RefCell::new(Layout::new()));
    let weak = viewport.as_weak();

    {
        let state = state.clone();
        let weak = weak.clone();
        global.on_reset(move |count| {
            state.borrow_mut().reset(count);
            if let Some(viewport) = weak.upgrade() {
                push_content(&PrivateListLayout::get(&viewport), &mut state.borrow_mut());
            }
        });
    }
    {
        let state = state.clone();
        let weak = weak.clone();
        global.on_set_count(move |count| {
            state.borrow_mut().set_count(count);
            if let Some(viewport) = weak.upgrade() {
                push_content(&PrivateListLayout::get(&viewport), &mut state.borrow_mut());
            }
        });
    }
    {
        let state = state.clone();
        let weak = weak.clone();
        global.on_set_height(move |index, height| {
            state.borrow_mut().set_height(index, height);
            if let Some(viewport) = weak.upgrade() {
                push_content(&PrivateListLayout::get(&viewport), &mut state.borrow_mut());
            }
        });
    }
    {
        let state = state.clone();
        let weak = weak.clone();
        global.on_set_viewport(move |offset, height| {
            state.borrow_mut().set_viewport(offset, height);
            if let Some(viewport) = weak.upgrade() {
                push_content(&PrivateListLayout::get(&viewport), &mut state.borrow_mut());
            }
        });
    }
}

fn setup_group(viewport: &Viewport) {
    let global = GroupListLayout::get(viewport);
    let state = Rc::new(RefCell::new(Layout::new()));
    let weak = viewport.as_weak();

    {
        let state = state.clone();
        let weak = weak.clone();
        global.on_reset(move |count| {
            state.borrow_mut().reset(count);
            if let Some(viewport) = weak.upgrade() {
                push_content_group(&GroupListLayout::get(&viewport), &mut state.borrow_mut());
            }
        });
    }
    {
        let state = state.clone();
        let weak = weak.clone();
        global.on_set_count(move |count| {
            state.borrow_mut().set_count(count);
            if let Some(viewport) = weak.upgrade() {
                push_content_group(&GroupListLayout::get(&viewport), &mut state.borrow_mut());
            }
        });
    }
    {
        let state = state.clone();
        let weak = weak.clone();
        global.on_set_height(move |index, height| {
            state.borrow_mut().set_height(index, height);
            if let Some(viewport) = weak.upgrade() {
                push_content_group(&GroupListLayout::get(&viewport), &mut state.borrow_mut());
            }
        });
    }
    {
        let state = state.clone();
        let weak = weak.clone();
        global.on_set_viewport(move |offset, height| {
            state.borrow_mut().set_viewport(offset, height);
            if let Some(viewport) = weak.upgrade() {
                push_content_group(&GroupListLayout::get(&viewport), &mut state.borrow_mut());
            }
        });
    }
}
