use egui::NumExt;

pub mod curve;

pub use curve::*;

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Default, PartialEq)]
enum DragTarget {
    LeftTangent,
    #[default]
    Handle,
    RightTangent,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Default)]
struct CurveEditorState {
    dragging: Option<DragTarget>,
    selected: Option<usize>,
}

impl CurveEditorState {
    pub fn load(ctx: &egui::Context, id: egui::Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ctx: &egui::Context, id: egui::Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }
}

pub struct CurveEditor<'a> {
    curve: &'a mut Curve,
    min_size: egui::Vec2,
    max_size: Option<egui::Vec2>,
    width: Option<f32>,
    height: Option<f32>,
    view_aspect: f32,
}

#[allow(unused)]
impl<'a> CurveEditor<'a> {
    pub fn new(curve: &'a mut Curve) -> Self {
        Self {
            curve,
            min_size: egui::vec2(40.0, 40.0),
            max_size: None,
            width: None,
            height: None,
            view_aspect: 13.0 / 6.0,
        }
    }

    pub fn with_min_size(self, min_size: egui::Vec2) -> Self {
        Self { min_size, ..self }
    }

    pub fn with_max_size(self, max_size: egui::Vec2) -> Self {
        Self {
            max_size: Some(max_size),
            ..self
        }
    }

    pub fn with_width(self, width: f32) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    pub fn with_height(self, height: f32) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }

    pub fn with_size(self, size: egui::Vec2) -> Self {
        Self {
            width: Some(size.x),
            height: Some(size.y),
            ..self
        }
    }

    pub fn with_aspect(self, view_aspect: f32) -> Self {
        Self {
            view_aspect,
            ..self
        }
    }

    fn load_state(ctx: &egui::Context, id: egui::Id) -> Option<CurveEditorState> {
        CurveEditorState::load(ctx, id)
    }

    fn store_state(ctx: &egui::Context, id: egui::Id, state: CurveEditorState) {
        state.store(ctx, id);
    }

    fn normalized_to_plot_coords(plot_rect: egui::Rect, coords: egui::Pos2) -> egui::Pos2 {
        plot_rect.lerp_inside(egui::vec2(coords.x, 1.0 - coords.y))
    }

    fn plot_to_normalized_coords(plot_rect: egui::Rect, coords: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            (coords.x - plot_rect.left()) / plot_rect.width(),
            1.0 - ((coords.y - plot_rect.top()) / plot_rect.height()),
        )
    }

    fn get_tangents_plot_coords(
        plot_rect: egui::Rect,
        pos: egui::Pos2,
        left: f32,
        right: f32,
    ) -> (egui::Pos2, egui::Pos2) {
        let left_dir = -egui::vec2(1.0, -left).normalized();
        let right_dir = egui::vec2(1.0, -right).normalized();

        let plot_pos = CurveEditor::normalized_to_plot_coords(plot_rect, pos);

        let plot_left = plot_pos + left_dir * 20.0;
        let plot_right = plot_pos + right_dir * 20.0;

        (plot_left, plot_right)
    }
}

impl<'a> egui::Widget for CurveEditor<'a> {
    // TODO: Make textual interface
    // TODO: Make sure tangents are always inside visible area?
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        // Determine position of widget.
        let pos = ui.available_rect_before_wrap().min;
        // Minimum values for screen protection
        let mut min_size = self.min_size;
        min_size.x = min_size.x.at_least(1.0);
        min_size.y = min_size.y.at_least(1.0);

        // Determine size of widget.
        let width = self.width;
        let height = self.height;
        let view_aspect = self.view_aspect;
        let size = {
            let mut width = width
                .unwrap_or_else(|| {
                    if let (Some(height), aspect) = (height, view_aspect) {
                        height * aspect
                    } else {
                        ui.available_size_before_wrap().x
                    }
                })
                .at_least(min_size.x);

            let mut height = height
                .unwrap_or_else(|| width / view_aspect)
                .at_least(min_size.y);

            if let Some(max_size) = self.max_size {
                width = width.min(max_size.x);
                height = height.min(max_size.y);
            }

            egui::vec2(width, height)
        };

        // Determine complete rect of widget.
        let complete_rect = egui::Rect {
            min: pos,
            max: pos + size,
        };

        let id = ui.next_auto_id();

        let plot_rect = complete_rect;
        let sense = egui::Sense::click_and_drag();
        let response = ui.allocate_rect(plot_rect, sense);

        // Load or initialize the memory.
        ui.ctx().check_for_id_clash(id, plot_rect, "Plot");

        let show_background = true;
        // Background
        if show_background {
            ui.painter()
                .with_clip_rect(plot_rect)
                .add(egui::epaint::RectShape::new(
                    plot_rect,
                    2,
                    ui.visuals().extreme_bg_color,
                    ui.visuals().widgets.noninteractive.bg_stroke,
                    egui::StrokeKind::Inside,
                ));
        }

        let mut state = CurveEditor::load_state(ui.ctx(), id).unwrap_or(CurveEditorState {
            dragging: None,
            selected: None,
        });

        if (response.clicked() || response.secondary_clicked() || response.dragged())
            && response.hover_pos().is_some()
            && state.dragging.is_none()
        {
            let pos = response.hover_pos().unwrap();

            let positions = self.curve.point_positions();
            let mut handles: Vec<_> = positions
                .iter()
                .enumerate()
                .map(|(index, &pos)| {
                    (
                        DragTarget::Handle,
                        index,
                        CurveEditor::normalized_to_plot_coords(plot_rect, pos),
                    )
                })
                .collect();

            if let Some(selected) = state.selected {
                let selected_pos = self
                    .curve
                    .get_position(selected)
                    .expect("Selected is invalid?");
                let left = self
                    .curve
                    .get_left_tan(selected)
                    .expect("Selected is invalid?");
                let right = self
                    .curve
                    .get_right_tan(selected)
                    .expect("Selected is invalid?");

                let (left_pos, right_pos) =
                    CurveEditor::get_tangents_plot_coords(plot_rect, selected_pos, left, right);

                handles.push((DragTarget::LeftTangent, selected, left_pos));
                handles.push((DragTarget::RightTangent, selected, right_pos));
            }

            let near = handles
                .iter()
                .find(|(_, _, handle_pos)| handle_pos.distance(pos).abs() < 15.0);

            // Add handle?
            if near.is_none() {
                if response.clicked_by(egui::PointerButton::Primary)
                    || response.dragged_by(egui::PointerButton::Primary)
                {
                    let index = self.curve.add_point(Point::from_pos(
                        CurveEditor::plot_to_normalized_coords(plot_rect, pos),
                    ));
                    state.selected = Some(index);
                }
            } else {
                let (drag_type, index, _) = near.unwrap();

                // Start dragging?
                if response.clicked_by(egui::PointerButton::Primary)
                    || response.dragged_by(egui::PointerButton::Primary)
                {
                    state.dragging = Some(*drag_type);
                    state.selected = Some(*index);
                }

                // Remove handle?
                if response.secondary_clicked()
                    && !self.curve.index_is_first_or_last(*index)
                    && *drag_type == DragTarget::Handle
                {
                    self.curve.remove_point(*index);
                    state.dragging = None;
                    state.selected = None;
                }
            }
        }

        // Stop dragging?
        if state.dragging.is_some()
            && (response.drag_stopped() || !response.is_pointer_button_down_on())
        {
            state.dragging = None;
        }

        // Handle dragging
        if let (Some(index), Some(drag_type)) = (state.selected, state.dragging) {
            // Handle dragging tangents also
            if let Some(pos) = self.curve.get_position(index) {
                match drag_type {
                    DragTarget::Handle => {
                        let screen_pos = (CurveEditor::normalized_to_plot_coords(plot_rect, pos)
                            + response.drag_delta())
                        .clamp(plot_rect.left_top(), plot_rect.right_bottom());

                        if !self.curve.index_is_first_or_last(index) {
                            self.curve.set_position(
                                index,
                                CurveEditor::plot_to_normalized_coords(plot_rect, screen_pos),
                            );
                        } else {
                            self.curve.set_position(
                                index,
                                egui::pos2(
                                    pos.x,
                                    CurveEditor::plot_to_normalized_coords(plot_rect, screen_pos).y,
                                ),
                            );
                        }
                    }
                    DragTarget::LeftTangent => {
                        if !self.curve.index_is_first(index) {
                            let screen_pos = CurveEditor::normalized_to_plot_coords(plot_rect, pos);
                            let tangent = self.curve.get_left_tan(index).unwrap();
                            let (plot_tangent, _) =
                                CurveEditor::get_tangents_plot_coords(plot_rect, pos, tangent, 0.0);

                            let mut screen_tan = plot_tangent + response.drag_delta();
                            screen_tan.x = screen_tan.x.min(screen_pos.x);

                            let tangent_dir = (screen_tan - screen_pos).normalized();

                            self.curve
                                .set_left_tan(index, -tangent_dir.y / tangent_dir.x);
                        }
                    }
                    DragTarget::RightTangent => {
                        if !self.curve.index_is_last(index) {
                            let screen_pos = CurveEditor::normalized_to_plot_coords(plot_rect, pos);
                            let tangent = self.curve.get_right_tan(index).unwrap();
                            let (_, plot_tangent) =
                                CurveEditor::get_tangents_plot_coords(plot_rect, pos, 0.0, tangent);

                            let mut screen_tan = plot_tangent + response.drag_delta();
                            screen_tan.x = screen_tan.x.max(screen_pos.x);

                            let tangent_dir = (screen_tan - screen_pos).normalized();

                            self.curve
                                .set_right_tan(index, -tangent_dir.y / tangent_dir.x);
                        }
                    }
                }
            }
        }

        let mut points = vec![];
        let mut offset = 0.0;
        let step = 0.001;
        while offset < 1.0 {
            points.push(egui::pos2(offset, self.curve.sample(offset)));

            offset += step;
        }
        ui.painter()
            .with_clip_rect(plot_rect)
            .add(egui::epaint::PathShape::line(
                points
                    .iter()
                    .map(|&pos| CurveEditor::normalized_to_plot_coords(plot_rect, pos))
                    .collect(),
                ui.visuals().widgets.noninteractive.fg_stroke,
            ));

        let visuals = ui.style().interact(&response);

        // Draw tangents
        if let Some(selected) = state.selected {
            let pos = self
                .curve
                .get_position(selected)
                .expect("Selected is invalid?");
            let left = self
                .curve
                .get_left_tan(selected)
                .expect("Selected is invalid?");
            let right = self
                .curve
                .get_right_tan(selected)
                .expect("Selected is invalid?");

            let plot_pos = CurveEditor::normalized_to_plot_coords(plot_rect, pos);
            let (plot_left, plot_right) =
                CurveEditor::get_tangents_plot_coords(plot_rect, pos, left, right);

            ui.painter()
                .with_clip_rect(plot_rect)
                .line_segment([plot_left, plot_pos], visuals.fg_stroke);
            ui.painter()
                .with_clip_rect(plot_rect)
                .line_segment([plot_right, plot_pos], visuals.fg_stroke);

            ui.painter()
                .with_clip_rect(plot_rect)
                .add(egui::epaint::CircleShape {
                    center: plot_left,
                    radius: 3.5,
                    fill: visuals.bg_fill,
                    stroke: visuals.fg_stroke,
                });
            ui.painter()
                .with_clip_rect(plot_rect)
                .add(egui::epaint::CircleShape {
                    center: plot_right,
                    radius: 3.5,
                    fill: visuals.bg_fill,
                    stroke: visuals.fg_stroke,
                });
        }

        for &handle_pos in self.curve.point_positions().iter() {
            ui.painter()
                .with_clip_rect(plot_rect)
                .add(egui::epaint::CircleShape {
                    center: CurveEditor::normalized_to_plot_coords(plot_rect, handle_pos),
                    radius: 5.0,
                    fill: visuals.bg_fill,
                    stroke: visuals.fg_stroke,
                });
        }

        CurveEditor::store_state(ui.ctx(), id, state);

        ui.advance_cursor_after_rect(complete_rect);

        response
    }
}
