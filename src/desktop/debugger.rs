use eframe::egui;
use ibniz::{Context, Instruction, MEMORY_WORDS, Vm};

const MEMORY_PAGE: usize = 128;

pub struct Debugger {
    pub open: bool,
    context: Option<Context>,
    stack_rows: usize,
    memory_address: usize,
}

impl Default for Debugger {
    fn default() -> Self {
        Self {
            open: false,
            context: None,
            stack_rows: 16,
            memory_address: 0,
        }
    }
}

pub fn instruction_label(instruction: Instruction) -> String {
    match instruction {
        Instruction::Immediate(value) => format!(
            "push {:08X} ({:.6})",
            value as u32,
            f64::from(value) / 65536.0
        ),
        Instruction::Op(op) => format!("{}", char::from(op)),
        Instruction::End => "implicit end of program".into(),
    }
}

impl Debugger {
    pub fn show(&mut self, ctx: &egui::Context, vm: &Vm, can_step: bool) -> bool {
        if !self.open {
            return false;
        }
        let mut open = self.open;
        let mut step = false;
        egui::Window::new("VM debugger").open(&mut open).default_width(820.0)
            .default_height(580.0).vscroll(true).show(ctx, |ui| {
                let status = vm.status();
                ui.horizontal(|ui| {
                    step = ui.add_enabled(can_step, egui::Button::new("Step (F3)")).clicked();
                    ui.label(format!("Active: {:?} | IP: {} | Executed: {}", status.context,
                        status.instruction_pointer, status.instructions_executed));
                });
                ui.label(format!("Next: {}{}", instruction_label(vm.current_instruction()),
                    if status.stopped { " (stopped; F2 to reset)" } else { "" }));
                ui.small("Step requires paused playback, valid source and no active/finishing recording. Steps are silent and follow normal audio/video scheduling, including tick boundaries.");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Inspect");
                    ui.selectable_value(&mut self.context, None, "Active");
                    ui.selectable_value(&mut self.context, Some(Context::Video), "Video");
                    ui.selectable_value(&mut self.context, Some(Context::Audio), "Audio");
                    ui.label("Slots");
                    ui.add(egui::DragValue::new(&mut self.stack_rows).range(1..=64));
                });
                let context = self.context.unwrap_or(status.context);
                ui.small("Cyclic slots, top first: no logical stack depth is tracked. Return slots contain raw addresses/counts and can include old values.");
                ui.columns(2, |columns| {
                    for (index, return_stack) in [false, true].into_iter().enumerate() {
                        let ui = &mut columns[index];
                        let pointer = match (context, return_stack) {
                            (Context::Video, false) => status.video_stack_pointer,
                            (Context::Audio, false) => status.audio_stack_pointer,
                            (Context::Video, true) => status.video_return_stack_pointer,
                            (Context::Audio, true) => status.audio_return_stack_pointer,
                        };
                        ui.heading(if return_stack { "Return stack" } else { "Data stack" });
                        ui.monospace(format!("{context:?} {}: {pointer:05X}", if return_stack { "RSP" } else { "SP" }));
                        egui::Grid::new(("debug-stack", index)).striped(true).show(ui, |ui| {
                            ui.label("Depth"); ui.label("Address"); ui.label("Hex value"); ui.end_row();
                            for depth in 0..self.stack_rows {
                                let entry = if return_stack { vm.return_stack_entry(context, depth) }
                                    else { vm.data_stack_entry(context, depth) };
                                ui.monospace(format!("{depth:02}"));
                                ui.monospace(format!("{:05X}", entry.address));
                                value(ui, entry.value);
                                ui.end_row();
                            }
                        });
                    }
                });
                ui.separator();
                ui.heading("Memory (read-only)");
                ui.horizontal(|ui| {
                    ui.label("Word address (hex)");
                    ui.add(egui::DragValue::new(&mut self.memory_address)
                        .range(0..=MEMORY_WORDS - 1).hexadecimal(5, false, true));
                    if ui.button("Previous page").clicked() { self.memory_address = self.memory_address.saturating_sub(MEMORY_PAGE); }
                    if ui.button("Next page").clicked() { self.memory_address = self.memory_address.saturating_add(MEMORY_PAGE).min(MEMORY_WORDS - 1); }
                });
                ui.horizontal(|ui| {
                    if ui.button("Go to SP").clicked() { self.memory_address = vm.data_stack_entry(context, 0).address; }
                    if ui.button("Go to RSP").clicked() { self.memory_address = vm.return_stack_entry(context, 0).address; }
                });
                ui.small("Physical word addresses 00000-FFFFF. Hover a value for signed and 16.16 interpretations; the selected address is highlighted.");
                let base = self.memory_address / MEMORY_PAGE * MEMORY_PAGE;
                egui::ScrollArea::horizontal().id_salt("debug-memory-scroll").show(ui, |ui| {
                    egui::Grid::new("debug-memory").striped(true).show(ui, |ui| {
                        ui.label("Address");
                        for column in 0..8 { ui.monospace(format!("+{column:X}")); }
                        ui.end_row();
                        for row in 0..MEMORY_PAGE / 8 {
                            let start = base + row * 8;
                            ui.monospace(format!("{start:05X}"));
                            for address in start..start + 8 {
                                let text = egui::RichText::new(format!("{:08X}", vm.memory()[address] as u32)).monospace();
                                let text = if address == self.memory_address {
                                    text.background_color(ui.visuals().selection.bg_fill)
                                } else { text };
                                ui.label(text).on_hover_text(format!(
                                    "Address {address:05X}\nSigned: {}\n16.16: {:.6}",
                                    vm.memory()[address], f64::from(vm.memory()[address]) / 65536.0,
                                ));
                            }
                            ui.end_row();
                        }
                    });
                });
            });
        self.open = open;
        step
    }
}

fn value(ui: &mut egui::Ui, value: i32) {
    ui.monospace(format!("{:08X}", value as u32))
        .on_hover_text(format!(
            "Signed: {value}\n16.16: {:.6}",
            f64::from(value) / 65536.0
        ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspecting_each_context_and_edge_memory_pages_does_not_execute_or_modify_the_vm() {
        let ctx = egui::Context::default();
        let mut vm = Vm::new(ibniz::Program::compile("1 P T").unwrap());
        vm.step();
        vm.step();
        let before = vm.status();
        let memory = vm.memory().to_vec();
        let mut debugger = Debugger {
            open: true,
            stack_rows: 1,
            ..Default::default()
        };
        for context in [None, Some(Context::Video), Some(Context::Audio)] {
            debugger.context = context;
            for address in [0, 0xe0003, MEMORY_WORDS - 1] {
                debugger.memory_address = address;
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    assert!(!debugger.show(ctx, &vm, false));
                });
                assert_eq!(debugger.memory_address, address);
            }
        }
        assert_eq!(vm.memory(), memory);
        assert_eq!(vm.status().instruction_pointer, before.instruction_pointer);
        assert_eq!(
            vm.status().instructions_executed,
            before.instructions_executed
        );
    }
}
