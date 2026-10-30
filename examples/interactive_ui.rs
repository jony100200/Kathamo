//! Public SDK proof: a real, interactive chat-shaped native UI, without a model backend.
//! cargo run -p kathamo --example interactive_ui -- --native
//! cargo run -p kathamo --example interactive_ui   (headless interaction assertions)

use kathamo::prelude::*;

pub struct InteractiveUi {
    pub document: DesignDocument,
    pub runtime: DesignRuntime,
    pub messages: Vec<String>,
    last_id: String,
}

impl InteractiveUi {
    pub fn new() -> Self {
        let mut document = DesignDocument::new("Messaging UI", 1200.0, 720.0);
        let r = |x, y, w, h| Rect::from_min_size(Vec2::new(x, y), Vec2::new(w, h));
        document.add(
            DesignKind::Panel,
            "Conversations",
            r(0.0, 0.0, 250.0, 720.0),
        );
        document.add(
            DesignKind::Label,
            "LOCAL UI DEMO",
            r(270.0, 10.0, 390.0, 40.0),
        );
        document.add(
            DesignKind::Label,
            "No model backend connected",
            r(270.0, 52.0, 480.0, 35.0),
        );
        let last = document.add(
            DesignKind::Label,
            "Write a message below",
            r(280.0, 150.0, 700.0, 55.0),
        );
        let prompt = document.add(
            DesignKind::TextInput,
            "Type a message",
            r(280.0, 650.0, 690.0, 44.0),
        );
        let send = document.add(DesignKind::Button, "Send", r(984.0, 650.0, 135.0, 44.0));
        let new_chat = document.add(
            DesignKind::Button,
            "New conversation",
            r(14.0, 70.0, 220.0, 40.0),
        );
        let toggle = document.add(
            DesignKind::Toggle,
            "Show timestamps",
            r(14.0, 135.0, 220.0, 35.0),
        );
        document
            .state_bindings
            .insert(prompt.clone(), "message.draft".into());
        document
            .state_bindings
            .insert(toggle, "settings.timestamps".into());
        document.action_bindings.insert(send, "message.send".into());
        document
            .action_bindings
            .insert(prompt, "message.send".into());
        document
            .action_bindings
            .insert(new_chat, "message.new".into());
        let mut runtime = DesignRuntime::new();
        runtime.set("message.draft", UiValue::Text(String::new()));
        runtime.set("settings.timestamps", UiValue::Bool(false));
        // Resize the composer with the window; anchor the Send button at bottom right.
        let composer = document
            .elements
            .iter()
            .find(|n| n.label == "Type a message")
            .map(|n| n.id.clone())
            .unwrap();
        runtime.responsive.insert(
            composer,
            ResponsiveRule::Anchored {
                left: true,
                right: true,
                top: false,
                bottom: true,
                min_size: Vec2::new(200.0, 44.0),
                max_size: None,
            },
        );
        let send_id = document
            .elements
            .iter()
            .find(|n| n.label == "Send")
            .map(|n| n.id.clone())
            .unwrap();
        runtime.responsive.insert(
            send_id,
            ResponsiveRule::Anchored {
                left: false,
                right: true,
                top: false,
                bottom: true,
                min_size: Vec2::new(135.0, 44.0),
                max_size: None,
            },
        );
        Self {
            document,
            runtime,
            messages: Vec::new(),
            last_id: last,
        }
    }

    fn consume_events(&mut self) {
        for event in self.runtime.take_events() {
            if event.kind != UiEventKind::Activated {
                continue;
            }
            match event.action_id.as_str() {
                "message.send" => {
                    let text = match self.runtime.get("message.draft") {
                        Some(UiValue::Text(v)) => v.trim().to_owned(),
                        _ => String::new(),
                    };
                    if !text.is_empty() {
                        self.messages.push(text.clone());
                        if let Some(node) = self.document.find_mut(&self.last_id) {
                            node.label = text;
                        }
                        self.runtime
                            .set("message.draft", UiValue::Text(String::new()));
                    }
                }
                "message.new" => {
                    self.messages.clear();
                    if let Some(node) = self.document.find_mut(&self.last_id) {
                        node.label = "Write a message below".into();
                    }
                    self.runtime
                        .set("message.draft", UiValue::Text(String::new()));
                }
                _ => {}
            }
        }
    }
}

impl Default for InteractiveUi {
    fn default() -> Self {
        Self::new()
    }
}

impl KathamoApp for InteractiveUi {
    fn title(&self) -> &str {
        "Kathamo: Interactive Messaging UI Example"
    }
    fn default_size(&self) -> (u32, u32) {
        (1200, 720)
    }

    fn update(&mut self, ctx: &mut AppContext) {
        self.consume_events();
        let bounds = ctx.bounds();
        self.runtime.render(
            &self.document,
            &mut ctx.cmd_buf,
            bounds,
            &ctx.pointer,
            &ctx.theme,
        );
    }

    fn handle_mouse_down(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        ctx: &mut AppContext,
    ) -> bool {
        self.runtime
            .pointer_down(&self.document, pos, button, &ctx.pointer)
    }
    fn handle_mouse_move(&mut self, pos: Vec2, delta: Vec2, _ctx: &mut AppContext) -> bool {
        self.runtime.pointer_move(pos, delta);
        true
    }
    fn handle_mouse_up(&mut self, pos: Vec2, button: PointerButton, _ctx: &mut AppContext) -> bool {
        self.runtime.pointer_up(&self.document, pos, button)
    }
    fn handle_scroll(&mut self, delta: Vec2, pos: Vec2, _ctx: &mut AppContext) -> bool {
        self.runtime.scroll(&self.document, delta, pos)
    }
    fn handle_key_down(&mut self, key: Key, mods: Modifiers, _ctx: &mut AppContext) -> bool {
        self.runtime.key_down(&self.document, key, mods)
    }
    fn handle_text_input(&mut self, text: &str, _ctx: &mut AppContext) -> bool {
        self.runtime.text_input(&self.document, text)
    }
    fn handle_ime(&mut self, event: ImeEvent, _ctx: &mut AppContext) -> bool {
        self.runtime.ime(&self.document, event)
    }
}

fn main() {
    if std::env::args().any(|arg| arg == "--native") {
        run_native(InteractiveUi::new()).expect("native UI example failed");
        return;
    }
    let mut runner = HeadlessAppRunner::new(InteractiveUi::new());
    assert!(runner.step_frame() > 0);
    let field = runner
        .app
        .document
        .elements
        .iter()
        .find(|n| n.kind == DesignKind::TextInput)
        .unwrap()
        .id
        .clone();
    runner.app.runtime.focused = Some(field);
    assert!(runner
        .app
        .runtime
        .text_input(&runner.app.document, "Hello বাংলা"));
    let button = runner
        .app
        .document
        .elements
        .iter()
        .find(|n| n.kind == DesignKind::Button && n.label == "Send")
        .unwrap();
    let click = button.bounds.center();
    let p = PointerState::default();
    runner
        .app
        .runtime
        .pointer_down(&runner.app.document, click, PointerButton::Primary, &p);
    runner
        .app
        .runtime
        .pointer_up(&runner.app.document, click, PointerButton::Primary);
    runner.step_frame();
    assert_eq!(runner.app.messages, vec!["Hello বাংলা".to_string()]);
    println!("Kathamo live UI: native widget bindings and UTF-8 input passed headless exercise");
}
