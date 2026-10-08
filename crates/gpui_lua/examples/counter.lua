-- Reactive Counter Example in GPUI-CE LuaJIT DSL
local count, set_count = signal(0, "counter_val")
http.get("https://www.fruityvice.com/api/fruit/banana", function(err, res)
    log.info(res.json().name)
end)

function App()
    return ui.div({
        w_full = true,
        h_full = true,
        items_center = true,
        justify_center = true,
        overflow_scroll = true,
        bg = colors.rgba(0,0,0,0.5),
        p = 40,
        children = {
            ui.div({
                w = 360,
                p = 24,
                bg = "#313244",
                rounded = 8,
                border = 1,
                border_color = "#313244",
                shadow_md = true,
                flex_col = true,
                gap = 12,
                children = {
                    ui.text("GPUI-CE LuaJIT Counter"):bold():size(18):color("#cdd6f4"),
                    ui.text("Reactive state with hot reload preservation"):size(13):color("#a6adc8"),
                    ui.div({
                        items_center = true,
                        justify_center = true,
                        py = 16,
                        children = {
                            ui.text(tostring(count())):size(48):bold():color("#b4befe")
                        }
                    }),
                    ui.row({
                        gap = 12,
                        justify = "center",
                        children = {
                            ui.div({
                                bg = "#313244",
                                px = 16,
                                py = 8,
                                rounded = 6,
                                cursor_pointer = true,
                                on_click = function() set_count(count() - 1) end,
                                children = { ui.text("- Decrement"):bold():size(14):color("#cdd6f4") }
                            }),
                            ui.div({
                                bg = "#3b82f6",
                                px = 16,
                                py = 8,
                                rounded = 6,
                                cursor_pointer = true,
                                on_click = function() set_count(count() + 1) end,
                                children = { ui.text("+ Increment"):bold():size(14):color("#ffffff") }
                            }),
                            ui.div({
                                bg = "transparent",
                                px = 16,
                                py = 8,
                                rounded = 6,
                                cursor_pointer = true,
                                on_click = function() set_count(0) end,
                                children = { ui.text("Reset"):bold():size(14):color("#a6adc8") }
                            })
                        }
                    }),
                }
            })
        }
    })
end
