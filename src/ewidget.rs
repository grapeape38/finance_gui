extern crate gtk;

use crate::component::{widget_call, WidgetInfo, MyWidgetInfo};
use std::marker::PhantomData;
use crate::gui::{AppPtr};
use std::collections::HashMap;
use EWidget::*;

use gtk::{prelude::*, Widget, Button, Container, Window, Label, Orientation};

pub type WidgetMap = HashMap<EWidget, MyWidgetInfo>;

macro_rules! c_map(
    { $($key:expr => $value:ty),+ } => {
        {
            let mut m = HashMap::new();
            $(
                m.insert($key, MyWidgetInfo::new(Box::new(Factory::<$value>::new())));
            )+
            m
        }
     };
);

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum EWidget {
    SignInButton,
    LoadingFrame,
    ErrorPage,
    SignInLabel,
    GetTransButton,
    SignedInFrame, 
    MainBox,
    MainWindow,
    TransColLabel,
    TransColBin,
    TransRow,
    TransBox,
    LabelFrame,
    AccountBox,
    SomeLabel
}

pub fn create_widgets() -> WidgetMap {
    c_map!(
        SignInButton => Button,
        LoadingFrame => gtk::Frame,
        SignedInFrame => gtk::Frame,
        ErrorPage => gtk::Frame,
        GetTransButton => Button,
        MainWindow => Window,
        MainBox => gtk::Box,
        TransColLabel => gtk::Label,
        TransColBin => gtk::Frame,
        TransRow => gtk::Box,
        TransBox => gtk::Box,
        LabelFrame => gtk::Frame,
        AccountBox => gtk::Box,
        SomeLabel => Label
    )
}

pub type WidgetKey = (EWidget, String);

pub trait ToKey {
    fn to_key(self) -> WidgetKey;
}

impl ToKey for EWidget {
    fn to_key(self) -> WidgetKey {
        (self, "".to_string())
    }
}

impl<S: ToString> ToKey for (EWidget, S) {
    fn to_key(self) -> WidgetKey {
        (self.0, self.1.to_string())
    }
}

pub trait WidgetFactory {
    fn make(&self, info: &WidgetInfo, app: &AppPtr) -> Widget;
}

pub struct Factory<W: WidgetExt> {
    phantom: PhantomData<W> 
}

impl<W: WidgetExt> Factory<W> {
    pub fn new() -> Self { Factory { phantom: PhantomData } }
}

impl WidgetFactory for Factory<Button> {
    fn make(&self, info: &WidgetInfo, app: &AppPtr) -> Widget {
        let button = match info.attributes.get("label") {
            Some(label) => Button::new_with_label(label),
            None => Button::new()
        };
        if let Some(callback) = info.callbacks.get("clicked") {
            button.connect_clicked(widget_call(callback, app));
        }
        button.upcast::<Widget>()
    }
}

impl WidgetFactory for Factory<Label> {
    fn make(&self, info: &WidgetInfo, _: &AppPtr) -> Widget {
        gtk::Label::new(info.attributes.get("text").map(|s| &s[..])).upcast::<Widget>()
    }
}

impl WidgetFactory for Factory<Window> {
    fn make(&self, _: &WidgetInfo, _: &AppPtr) -> Widget {
        Window::new(gtk::WindowType::Toplevel).upcast::<Widget>()
    }
}

impl WidgetFactory for Factory<gtk::Box> {
    fn make(&self, info: &WidgetInfo, _: &AppPtr) -> Widget {
        let orientation = info.attributes.get("orientation")
            .map(|s| if s == "vertical" { Orientation::Vertical } else { Orientation::Horizontal })
            .unwrap_or(Orientation::Vertical);
        let spacing = info.attributes.get("spacing")
            .map(|s| s.parse::<i32>().unwrap_or(10)).unwrap_or(10);
        gtk::Box::new(orientation, spacing).upcast::<Widget>()
    }
}

impl WidgetFactory for Factory<gtk::Frame> {
    fn make(&self, info: &WidgetInfo, _: &AppPtr) -> Widget {
        gtk::Frame::new(info.attributes.get("label").map(|s| &s[..])).upcast::<Widget>()
    }
}

/*impl WidgetFactory for Factory<gtk::Bin> {
    fn make(&self, info: &WidgetInfo, _: &AppPtr) -> Widget {

    }
}*/

/*trait FWidgetExt {
    fn f_add(&self, child_widget: &Widget, parent_info: &WidgetInfo, child_info: &WidgetInfo);
    fn f_remove(&self, child_widget: &Widget, parent_info: &WidgetInfo, child_info: &WidgetInfo);
}

impl FWidgetExt for Container {
    fn f_add(&self, widget: &Widget, _: &WidgetInfo) {
        self.add(widget);
    }
    fn f_remove(&self, widget: &Widget, _: &WidgetInfo) {
        self.remove(widget);
    }
}*/
