---@meta
local components = {}

---Row flex layout component
---@param props RowComponentProps
---@return LuaElementBuilder
function components.row(props)
    local p = props or {}
    local row = ui.div():flex_row():items_center()

    if p.gap then row:gap(p.gap) end
    if p.align == "start" then row:items_start()
    elseif p.align == "end" then row:items_end()
    elseif p.align == "center" then row:items_center() end

    if p.justify == "start" then row:justify_start()
    elseif p.justify == "center" then row:justify_center()
    elseif p.justify == "end" then row:justify_end()
    elseif p.justify == "between" then row:justify_between() end

    if p.p then row:p(p.p) end
    if p.w_full then row:w_full() end

    if p.children then
        row:children(p.children)
    end
    for i = 1, #p do
        if p[i] then
            row:child(p[i])
        end
    end

    return row
end

---Column flex layout component
---@param props ColumnComponentProps
---@return LuaElementBuilder
function components.column(props)
    local p = props or {}
    local col = ui.div():flex_col()

    if p.gap then col:gap(p.gap) end
    if p.align == "start" then col:items_start()
    elseif p.align == "end" then col:items_end()
    elseif p.align == "center" then col:items_center() end

    if p.p then col:p(p.p) end
    if p.w_full then col:w_full() end

    if p.children then
        col:children(p.children)
    end
    for i = 1, #p do
        if p[i] then
            col:child(p[i])
        end
    end

    return col
end
---Stack overlay layout container
---@param props StackComponentProps
---@return LuaElementBuilder
function components.stack(props)
    local p = props or {}
    local stack = ui.div()
    if p.w_full then stack:w_full() end
    if p.h_full then stack:h_full() end

    if p.children then
        stack:children(p.children)
    end
    for i = 1, #p do
        if p[i] then
            stack:child(p[i])
        end
    end

    return stack
end

return components
