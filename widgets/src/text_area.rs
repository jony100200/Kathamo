//! Reusable multiline text surface: shaped text, caret, selection and navigation.
//! Consumers own documents, undo history, clipboard and persistence.
use ui_core::{CornerRadius, Rect, Stroke, TextEditBuffer, Vec2};
use ui_platform::{Key, Modifiers};
use ui_render::{measure_text, CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;

const FONT: f32 = 14.0;
const ROW: f32 = 23.0;
const GUTTER: f32 = 58.0;
const VISIBLE_COLUMNS: usize = 1024;

#[derive(Clone, Debug)]
struct Line { byte: usize, character: usize, length: usize }

#[derive(Clone, Debug)]
pub struct TextArea {
    pub buffer: TextEditBuffer,
    pub scroll_row: usize,
    pub scroll_column: usize,
    pub preedit: String,
    lines: Vec<Line>,
    revision: u64,
    dragging: bool,
    preferred_column: Option<usize>,
    visible_rows: usize,
    visible_columns: usize,
}

impl Default for TextArea {
    fn default() -> Self {
        Self { buffer: TextEditBuffer::default(), scroll_row: 0, scroll_column: 0, preedit: String::new(),
            lines: vec![], revision: u64::MAX, dragging: false, preferred_column: None, visible_rows: 20, visible_columns: 80 }
    }
}

impl TextArea {
    fn index(&mut self) {
        if self.revision == self.buffer.revision() { return; }
        self.lines.clear();
        let mut byte = 0; let mut character = 0;
        for line in self.buffer.as_str().split('\n') {
            let length = line.chars().count();
            self.lines.push(Line { byte, character, length });
            byte += line.len() + 1; character += length + 1;
        }
        self.revision = self.buffer.revision();
        self.scroll_row = self.scroll_row.min(self.lines.len().saturating_sub(1));
    }
    pub fn line_column(&mut self) -> (usize, usize) {
        self.index();
        let row = self.lines.partition_point(|l| l.character <= self.buffer.cursor()).saturating_sub(1);
        (row, self.buffer.cursor().saturating_sub(self.lines[row].character))
    }
    fn row_text(&self, row: usize) -> &str {
        let start = self.lines[row].byte;
        let end = self.lines.get(row + 1).map_or(self.buffer.as_str().len(), |l| l.byte - 1);
        &self.buffer.as_str()[start..end]
    }
    fn text_rect(bounds: Rect) -> Rect {
        Rect::from_min_max(bounds.min + Vec2::new(GUTTER, 8.0), bounds.max - Vec2::new(8.0, 8.0))
    }
    fn visible_text(&self, row: usize) -> &str {
        let text = self.row_text(row);
        let begin = byte_at(text, self.scroll_column);
        let end = begin + byte_at(&text[begin..], VISIBLE_COLUMNS);
        &text[begin..end]
    }
    fn ensure_caret(&mut self) {
        let (row,column) = self.line_column();
        if row < self.scroll_row { self.scroll_row = row; }
        if row >= self.scroll_row + self.visible_rows { self.scroll_row = row + 1 - self.visible_rows; }
        if column < self.scroll_column { self.scroll_column = column; }
        if column >= self.scroll_column + self.visible_columns { self.scroll_column = column + 1 - self.visible_columns; }
    }
    fn hit(&mut self, pos: Vec2, bounds: Rect) -> usize {
        self.index();
        let rect = Self::text_rect(bounds);
        let row = (self.scroll_row + ((pos.y - rect.min.y).max(0.0) / ROW) as usize).min(self.lines.len() - 1);
        let text = self.visible_text(row);
        let target = (pos.x - rect.min.x).max(0.0);
        let mut low=0; let mut high=text.chars().count();
        while low<high {
            let mid=(low+high)/2;
            if measure_text(&text[..byte_at(text,mid)],FONT,false)<target {low=mid+1;} else {high=mid;}
        }
        if low>0 {
            let left=measure_text(&text[..byte_at(text,low-1)],FONT,false);
            let right=measure_text(&text[..byte_at(text,low)],FONT,false);
            if target-left < right-target {low-=1;}
        }
        self.lines[row].character + (self.scroll_column + low).min(self.lines[row].length)
    }
    pub fn pointer_down(&mut self, pos: Vec2, bounds: Rect, extend: bool) -> bool {
        if !bounds.contains(pos) { return false; }
        let caret=self.hit(pos,bounds);
        self.buffer.set_focused(true); self.buffer.set_cursor(caret,extend);
        self.dragging=true; self.preferred_column=None; true
    }
    pub fn pointer_move(&mut self, pos: Vec2, bounds: Rect) -> bool {
        if !self.dragging {return false;}
        if pos.y < bounds.min.y {self.scroll_row=self.scroll_row.saturating_sub(1);}
        if pos.y > bounds.max.y {self.scroll_row=(self.scroll_row+1).min(self.lines.len().saturating_sub(1));}
        let caret=self.hit(pos,bounds);self.buffer.set_cursor(caret,true);true
    }
    pub fn pointer_up(&mut self) { self.dragging=false; }
    pub fn scroll(&mut self, delta: Vec2) {
        self.index();
        if delta.y<0.0 { self.scroll_row=(self.scroll_row+3).min(self.lines.len().saturating_sub(1)); }
        else if delta.y>0.0 {self.scroll_row=self.scroll_row.saturating_sub(3);}
        if delta.x<0.0 {self.scroll_column=self.scroll_column.saturating_add(4);}
        else if delta.x>0.0 {self.scroll_column=self.scroll_column.saturating_sub(4);}
    }
    pub fn insert(&mut self,text:&str) {
        self.buffer.insert_str(text);self.preedit.clear();self.preferred_column=None;self.ensure_caret();
    }
    pub fn key(&mut self, key:Key, modifiers:Modifiers)->bool {
        self.index();let (row,column)=self.line_column();
        let vertical=matches!(key,Key::ArrowUp|Key::ArrowDown) || matches!(&key,Key::Custom(s) if s=="PageUp"||s=="PageDown");
        match key {
            Key::Character('a'|'A') if modifiers.ctrl=>self.buffer.select_all(),
            Key::ArrowLeft=>self.buffer.move_left(modifiers.shift),
            Key::ArrowRight=>self.buffer.move_right(modifiers.shift),
            Key::ArrowUp|Key::ArrowDown=>{
                let preferred=*self.preferred_column.get_or_insert(column);
                let target=if key==Key::ArrowUp {row.saturating_sub(1)} else {(row+1).min(self.lines.len()-1)};
                self.buffer.set_cursor(self.lines[target].character+preferred.min(self.lines[target].length),modifiers.shift);
            }
            Key::Custom(ref name) if name=="Home"||name=="End"=>{
                let target=if modifiers.ctrl {if name=="Home" {0}else{self.buffer.char_len()}}
                    else {self.lines[row].character+if name=="Home"{0}else{self.lines[row].length}};
                self.buffer.set_cursor(target,modifiers.shift);
            }
            Key::Custom(ref name) if name=="PageUp"||name=="PageDown"=>{
                let target=if name=="PageUp"{row.saturating_sub(self.visible_rows)}else{(row+self.visible_rows).min(self.lines.len()-1)};
                self.buffer.set_cursor(self.lines[target].character+column.min(self.lines[target].length),modifiers.shift);
            }
            Key::Backspace=>{self.buffer.backspace();}
            Key::Delete=>{self.buffer.delete();}
            Key::Enter=>self.buffer.insert_str("\n"),
            Key::Tab=>self.buffer.insert_str("    "),
            _=>return false,
        }
        if !vertical {self.preferred_column=None;}
        self.ensure_caret();true
    }
    pub fn render(&mut self, commands:&mut CommandBuffer,bounds:Rect,palette:&ThemePalette) {
        self.index();let rect=Self::text_rect(bounds);
        self.visible_rows=((rect.height()/ROW).floor() as usize).max(1);
        self.visible_columns=((rect.width()/9.0).floor() as usize).clamp(1,VISIBLE_COLUMNS/2);
        commands.push(DrawCommand::Rect{rect:bounds,fill:palette.canvas,radius:CornerRadius::same(4.0),stroke:Stroke::new(1.0,if self.buffer.is_focused(){palette.accent}else{palette.border})});
        commands.push(DrawCommand::ClipPush{rect:bounds});
        let selection=self.buffer.selection_range();
        let (caret_row,caret_column)=self.line_column();
        for row in self.scroll_row..(self.scroll_row+self.visible_rows+1).min(self.lines.len()) {
            let y=rect.min.y+(row-self.scroll_row) as f32*ROW;
            let visible=self.visible_text(row);
            if let Some((start,end))=selection {
                let line=&self.lines[row];
                if start<=line.character+line.length && end>line.character {
                    let left=start.saturating_sub(line.character).saturating_sub(self.scroll_column);
                    let right=end.saturating_sub(line.character).saturating_sub(self.scroll_column);
                    let x1=rect.min.x+measure_text(&visible[..byte_at(visible,left)],FONT,false);
                    let x2=if end>line.character+line.length {rect.max.x}else{rect.min.x+measure_text(&visible[..byte_at(visible,right)],FONT,false)};
                    commands.push(DrawCommand::Rect{rect:Rect::from_min_size(Vec2::new(x1,y-3.0),Vec2::new((x2-x1).max(2.0),ROW)),fill:palette.accent.with_alpha(50),radius:CornerRadius::ZERO,stroke:Stroke::NONE});
                }
            }
            draw_text(commands,format!("{}",row+1),Vec2::new(bounds.min.x+10.0,y),12.0,palette.text_muted);
            commands.push(DrawCommand::ClipPush{rect});
            draw_text(commands,visible.to_owned(),Vec2::new(rect.min.x,y),FONT,palette.text);
            if self.buffer.is_focused() && row==caret_row {
                let x=rect.min.x+measure_text(&visible[..byte_at(visible,caret_column.saturating_sub(self.scroll_column))],FONT,false);
                commands.push(DrawCommand::Line{start:Vec2::new(x,y-2.0),end:Vec2::new(x,y+17.0),stroke:Stroke::new(1.5,palette.accent)});
                if !self.preedit.is_empty(){draw_text(commands,self.preedit.clone(),Vec2::new(x,y+2.0),FONT,palette.accent);}
            }
            commands.push(DrawCommand::ClipPop);
        }
        commands.push(DrawCommand::ClipPop);
    }
}

fn byte_at(text:&str,character:usize)->usize {text.char_indices().nth(character).map_or(text.len(),|(byte,_)|byte)}
fn draw_text(commands:&mut CommandBuffer,text:String,pos:Vec2,size:f32,color:ui_core::Color) {
    commands.push(DrawCommand::Text{text,pos,size,color,strong:false});
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_caret_drag_selection_navigation_and_trailing_line() {
        let mut area=TextArea::default();area.buffer.set_text("abc\nবাংলা\nx\n");
        area.buffer.set_cursor(2,false);
        area.key(Key::ArrowDown,Modifiers::default());assert_eq!(area.line_column(),(1,2));
        area.key(Key::Custom("End".into()),Modifiers::default());assert_eq!(area.line_column(),(1,5));
        area.key(Key::ArrowDown,Modifiers::default());assert_eq!(area.line_column(),(2,1));
        area.key(Key::ArrowUp,Modifiers::default());assert_eq!(area.line_column(),(1,5));
        let bounds=Rect::from_min_size(Vec2::ZERO,Vec2::new(500.0,200.0));
        area.pointer_down(Vec2::new(GUTTER,8.0),bounds,false);
        area.pointer_move(Vec2::new(GUTTER+measure_text("ab",FONT,false),8.0),bounds);area.pointer_up();
        assert_eq!(area.buffer.selected_text(),Some("ab"));
        area.insert("ক");assert!(area.buffer.as_str().starts_with("কc\n"));
        area.key(Key::Custom("End".into()),Modifiers{ctrl:true,..Modifiers::default()});
        assert_eq!(area.line_column(),(3,0));
    }
}
