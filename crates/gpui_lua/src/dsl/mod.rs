pub mod builder;
pub mod convert;
pub mod node;
pub mod custom;
pub use builder::{
    parse_canvas_table, parse_div_table, parse_img_table, parse_svg_table, parse_text_table,
    LuaElementBuilder,
};
pub use convert::{convert_node, LuaInvoker};
pub use node::{
    AnimationProps, CanvasNode, ColorSpec, DivNode, Edges, ImgNode, Length, LuaNode, ShadowLevel,
    StyleProps, SvgNode, SyncRegistryKey, TextNode, TransitionsProps,
};
pub use custom::{
    bind_custom_element_in_lua, custom_element_names, get_custom_element, register_custom_element,
    CustomElementContext, CustomElementRenderer,
};

use mlua::{Function, Lua, Result, Value};

pub fn register(lua: &Lua) -> Result<()> {
    let ui = lua.create_table();

    // ui.div([props_or_children])
    ui.set(
        "div",
        lua.create_function(|lua, arg: Option<Value>| match arg {
            None => Ok(LuaElementBuilder::new_div()),
            Some(Value::Table(t)) => parse_div_table(lua, t),
            Some(Value::UserData(ud)) => {
                let div = LuaElementBuilder::new_div();
                if let Ok(child_builder) = ud.borrow::<LuaElementBuilder>() {
                    div.with_div_mut(|d| d.children.push(child_builder.to_node()));
                }
                Ok(div)
            }
            Some(Value::String(s)) => {
                let div = LuaElementBuilder::new_div();
                let text = s.to_str().map(|v| v.to_string()).unwrap_or_default();
                div.with_div_mut(|d| {
                    d.children.push(LuaNode::Text(TextNode {
                        content: text,
                        ..Default::default()
                    }))
                });
                Ok(div)
            }
            _ => Ok(LuaElementBuilder::new_div()),
        })?,
    )?;

    // ui.text(str_or_props)
    ui.set(
        "text",
        lua.create_function(|_lua, arg: Value| match arg {
            Value::String(s) => {
                let content = s.to_str()?.to_string();
                Ok(LuaElementBuilder::new_text(content))
            }
            Value::Table(t) => parse_text_table(t),
            Value::Integer(n) => Ok(LuaElementBuilder::new_text(n.to_string())),
            Value::Number(n) => Ok(LuaElementBuilder::new_text(n.to_string())),
            Value::Boolean(b) => Ok(LuaElementBuilder::new_text(b.to_string())),
            _ => Ok(LuaElementBuilder::new_text(String::new())),
        })?,
    )?;

    // ui.button(label, [on_click])
    ui.set(
        "button",
        lua.create_function(|lua, (label, on_click): (Value, Option<Function>)| {
            let label_str = match label {
                Value::String(s) => s.to_str()?.to_string(),
                Value::Integer(n) => n.to_string(),
                Value::Number(n) => n.to_string(),
                _ => "Button".to_string(),
            };

            let div = LuaElementBuilder::new_div();
            let click_key = match on_click {
                Some(f) => Some(SyncRegistryKey::new(lua.create_registry_value(f)?)),
                None => None,
            };

            let text_node = TextNode {
                content: label_str,
                bold: true,
                color: Some(ColorSpec::Hex("#cdd6f4".to_string())),
                ..Default::default()
            };

            div.with_div_mut(|d| {
                d.style.background = Some(ColorSpec::Hex("#313244".to_string()));
                d.style.text_color = Some(ColorSpec::Hex("#cdd6f4".to_string()));
                d.style.padding.left = Some(Length::Px(16.0));
                d.style.padding.right = Some(Length::Px(16.0));
                d.style.padding.top = Some(Length::Px(8.0));
                d.style.padding.bottom = Some(Length::Px(8.0));
                d.style.corner_radius = Some(6.0);
                d.style.cursor_pointer = true;
                d.style.items = Some(gpui::AlignItems::Center);
                d.style.justify = Some(gpui::JustifyContent::Center);
                d.on_click = click_key;
                d.children.push(LuaNode::Text(text_node));
            });

            Ok(div)
        })?,
    )?;

    // ui.svg(path_or_props)
    ui.set(
        "svg",
        lua.create_function(|_lua, arg: Value| match arg {
            Value::String(s) => {
                let path = s.to_str()?.to_string();
                Ok(LuaElementBuilder::new_svg(Some(path), None))
            }
            Value::Table(t) => parse_svg_table(t),
            _ => Ok(LuaElementBuilder::new_svg(None, None)),
        })?,
    )?;

    // ui.img(src_or_props)
    ui.set(
        "img",
        lua.create_function(|_lua, arg: Value| match arg {
            Value::String(s) => {
                let src = s.to_str()?.to_string();
                Ok(LuaElementBuilder::new_img(src, None))
            }
            Value::Table(t) => parse_img_table(t),
            _ => Ok(LuaElementBuilder::new_img(String::new(), None)),
        })?,
    )?;

    // ui.canvas(paint_fn_or_props)
    ui.set(
        "canvas",
        lua.create_function(|lua, arg: Value| match arg {
            Value::Function(f) => {
                let key = SyncRegistryKey::new(lua.create_registry_value(f)?);
                Ok(LuaElementBuilder::new_canvas(Some(key)))
            }
            Value::Table(t) => parse_canvas_table(lua, t),
            _ => Ok(LuaElementBuilder::new_canvas(None)),
        })?,
    )?;

    // ui.video(src_or_props)
    ui.set(
        "video",
        lua.create_function(|_lua, arg: Value| match arg {
            Value::String(s) => {
                let src = s.to_str()?.to_string();
                Ok(builder::LuaElementBuilder::new_video(src, None))
            }
            Value::Table(t) => builder::parse_video_table(t),
            Value::UserData(ud) => {
                let src = builder::extract_media_source_string(Value::UserData(ud)).unwrap_or_default();
                Ok(builder::LuaElementBuilder::new_video(src, None))
            }
            _ => Ok(builder::LuaElementBuilder::new_video(String::new(), None)),
        })?,
    )?;

    // ui.input(props)
    ui.set(
        "input",
        lua.create_function(|lua, arg: Value| match arg {
            Value::Table(t) => builder::parse_input_table(lua, t, false),
            Value::String(s) => {
                let t = lua.create_table();
                t.set("placeholder", s)?;
                builder::parse_input_table(lua, t, false)
            }
            _ => {
                let t = lua.create_table();
                builder::parse_input_table(lua, t, false)
            }
        })?,
    )?;
    ui.set("Input", ui.get::<Value>("input")?)?;

    // ui.textarea(props)
    ui.set(
        "textarea",
        lua.create_function(|lua, arg: Value| match arg {
            Value::Table(t) => builder::parse_input_table(lua, t, true),
            Value::String(s) => {
                let t = lua.create_table();
                t.set("placeholder", s)?;
                builder::parse_input_table(lua, t, true)
            }
            _ => {
                let t = lua.create_table();
                builder::parse_input_table(lua, t, true)
            }
        })?,
    )?;
    ui.set("Textarea", ui.get::<Value>("textarea")?)?;
    lua.globals().set("ui", ui)?;

    // Bind any custom elements registered on Rust side
    for name in custom_element_names() {
        let _ = bind_custom_element_in_lua(lua, &name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsl_method_chaining() -> mlua::Result<()> {
        let lua = Lua::new();
        register(&lua)?;
        let result: LuaElementBuilder = lua.load(r##"
            return ui.div():id("root"):flex_col():p(16):bg("#1e1e2e"):child(
                ui.text("Hello World"):size(20):bold()
            )
        "##).eval()?;
        let node = result.to_node();
        if let LuaNode::Div(div) = node {
            assert_eq!(div.id.as_deref(), Some("root"));
            assert_eq!(div.style.flex, Some(gpui::FlexDirection::Column));
            assert_eq!(div.style.padding.top, Some(Length::Px(16.0)));
            assert_eq!(div.style.background, Some(ColorSpec::Hex("#1e1e2e".to_string())));
            assert_eq!(div.children.len(), 1);
            if let LuaNode::Text(text) = &div.children[0] {
                assert_eq!(text.content, "Hello World");
                assert_eq!(text.size, Some(20.0));
                assert!(text.bold);
            } else {
                panic!("Expected text child");
            }
        } else {
            panic!("Expected div node");
        }

        Ok(())
    }

    #[test]
    fn test_dsl_table_syntax() -> mlua::Result<()> {
        let lua = Lua::new();
        register(&lua)?;

        let result: LuaElementBuilder = lua.load(r#"
            return ui.div({
                id = "box",
                flex_row = true,
                items_center = true,
                gap = 12,
                px = 10,
                py = 5,
                w_full = true,
                rounded = 8,
                shadow_md = true,
                cursor_pointer = true,
                children = {
                    ui.text("Child 1"),
                    ui.button("Click Me")
                }
            })
        "#).eval()?;

        let node = result.to_node();
        if let LuaNode::Div(div) = node {
            assert_eq!(div.id.as_deref(), Some("box"));
            assert_eq!(div.style.flex, Some(gpui::FlexDirection::Row));
            assert_eq!(div.style.items, Some(gpui::AlignItems::Center));
            assert_eq!(div.style.gap, Some(12.0));
            assert_eq!(div.style.padding.left, Some(Length::Px(10.0)));
            assert_eq!(div.style.padding.top, Some(Length::Px(5.0)));
            assert_eq!(div.style.width, Some(Length::Full));
            assert_eq!(div.style.corner_radius, Some(8.0));
            assert_eq!(div.style.shadow, Some(ShadowLevel::Md));
            assert!(div.style.cursor_pointer);
            assert_eq!(div.children.len(), 2);
        } else {
            panic!("Expected div node");
        }

        Ok(())
    }

    #[test]
    fn test_convert_to_any_element() -> mlua::Result<()> {
        let lua = Lua::new();
        register(&lua)?;

        let result: LuaElementBuilder = lua.load(r##"
            return ui.div():flex_col():p(10):bg("#1e1e2e"):child(
                ui.text("Test String"):size(14)
            )
        "##).eval()?;

        let node = result.to_node();
        let _element = convert_node(node, None);
        Ok(())
    }

    #[test]
    fn test_gpui_ce_expanded_styling() -> mlua::Result<()> {
        let lua = Lua::new();
        register(&lua)?;

        let result: LuaElementBuilder = lua.load(r##"
            return ui.div()
                :grid_cols(3)
                :col_span(2)
                :blur(10)
                :backdrop_blur(8)
                :aspect_square()
                :rounded_smoothing_ios()
                :border_dashed()
                :border_dashed_length(4)
                :border_dashed_gap(2)
                :overflow_hidden()
                :overflow_fade_x(16)
                :cursor_pointer()
                :flex_1()
                :self_center()
                :truncate()
                :underline()
                :letter_spacing(1.5)
                :child(ui.text("Decorated Text"):truncate():underline())
        "##).eval()?;

        let node = result.to_node();
        if let LuaNode::Div(div) = &node {
            assert!(div.style.grid);
            assert_eq!(div.style.grid_cols, Some(3));
            assert_eq!(div.style.col_span, Some(2));
            assert_eq!(div.style.blur, Some(10.0));
            assert_eq!(div.style.backdrop_blur, Some(8.0));
            assert!(div.style.aspect_square);
            assert!(div.style.rounded_smoothing_ios);
            assert!(div.style.border_dashed);
            assert_eq!(div.style.border_dashed_length, Some(4.0));
            assert_eq!(div.style.border_dashed_gap, Some(2.0));
            assert!(div.style.overflow_hidden);
            assert_eq!(div.style.overflow_fade.left, Some(16.0));
            assert_eq!(div.style.overflow_fade.right, Some(16.0));
            assert!(div.style.cursor_pointer);
            assert!(div.style.flex_1);
            assert!(div.style.truncate);
            assert!(div.style.underline);
            assert_eq!(div.style.letter_spacing, Some(1.5));
        } else {
            panic!("Expected div node");
        }

        let _element = convert_node(node, None);
        Ok(())
    }

    #[test]
    fn test_svg_img_canvas_and_animations() -> mlua::Result<()> {
        let lua = Lua::new();
        register(&lua)?;

        // 1. Svg test
        let svg_res: LuaElementBuilder = lua.load(r##"
            return ui.svg("icons/star.svg"):size(24, 24):text_color("#f9e2af")
        "##).eval()?;
        let svg_node = svg_res.to_node();
        if let LuaNode::Svg(s) = &svg_node {
            assert_eq!(s.path.as_deref(), Some("icons/star.svg"));
            assert_eq!(s.style.width, Some(Length::Px(24.0)));
            assert_eq!(s.style.height, Some(Length::Px(24.0)));
        } else {
            panic!("Expected Svg node");
        }
        let _svg_el = convert_node(svg_node, None);

        // 2. Img test
        let img_res: LuaElementBuilder = lua.load(r##"
            return ui.img({ src = "https://example.com/pic.png", fit = "cover", w = 120, h = 80, rounded = 8 })
        "##).eval()?;
        let img_node = img_res.to_node();
        if let LuaNode::Img(i) = &img_node {
            assert_eq!(i.src, "https://example.com/pic.png");
            assert_eq!(i.fit, Some(node::ImageFit::Cover));
            assert_eq!(i.style.width, Some(Length::Px(120.0)));
            assert_eq!(i.style.corner_radius, Some(8.0));
        } else {
            panic!("Expected Img node");
        }
        let _img_el = convert_node(img_node, None);

        // 3. Canvas test
        let canvas_res: LuaElementBuilder = lua.load(r##"
            return ui.canvas(function(bounds) end):size(200, 200)
        "##).eval()?;
        let canvas_node = canvas_res.to_node();
        if let LuaNode::Canvas(c) = &canvas_node {
            assert!(c.paint.is_some());
            assert_eq!(c.style.width, Some(Length::Px(200.0)));
        } else {
            panic!("Expected Canvas node");
        }
        let _canvas_el = convert_node(canvas_node, None);

        // 4. Transitions & Animations test
        let animated_res: LuaElementBuilder = lua.load(r##"
            return ui.div()
                :transition({ opacity = 300, bg = 200, all = 250 })
                :animate(1500, true, "ease_in_out")
        "##).eval()?;
        let anim_node = animated_res.to_node();
        if let LuaNode::Div(d) = &anim_node {
            let trans = d.transitions.expect("Expected transitions");
            assert_eq!(trans.all, Some(250));
            assert_eq!(trans.opacity, Some(300));
            let anim = d.animation.as_ref().expect("Expected animation");
            assert_eq!(anim.duration_ms, 1500);
            assert!(anim.repeat);
            assert_eq!(anim.easing.as_deref(), Some("ease_in_out"));
        } else {
            panic!("Expected Div node with animation");
        }
        let _anim_el = convert_node(anim_node, None);

        Ok(())
    }

    #[test]
    fn test_custom_element_registration() -> mlua::Result<()> {
        use gpui::{div, IntoElement, ParentElement};
        use std::sync::Arc;
        let lua = Lua::new();
        register_custom_element("my_badge", Arc::new(|cx| {
            let label = cx.get_str("label").unwrap_or("Default");
            div().child(label.to_string()).into_any_element()
        }));

        register(&lua)?;

        let result: LuaElementBuilder = lua.load(r#"
            return ui.my_badge({ label = "Special Badge", count = 42 }):child(ui.text("Child"))
        "#).eval()?;

        let node = result.to_node();
        if let LuaNode::Custom(custom) = node {
            assert_eq!(custom.tag, "my_badge");
            assert_eq!(custom.props.get("label").and_then(|v| v.as_str()), Some("Special Badge"));
            assert_eq!(custom.props.get("count").and_then(|v| v.as_i64()), Some(42));
            assert_eq!(custom.children.len(), 1);

            let any_element = convert_node(LuaNode::Custom(custom), None);
            let _ = any_element;
        } else {
            panic!("Expected custom node");
        }

        Ok(())
    }
}
