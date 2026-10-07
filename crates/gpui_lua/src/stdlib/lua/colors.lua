---@meta
local colors = {}

---Normalize or prepend # to hex color
---@param code string
---@return string
function colors.hex(code)
    if string.sub(code, 1, 1) ~= "#" then
        return "#" .. code
    end
    return code
end

---Format RGB to hex color string
---@param r integer
---@param g integer
---@param b integer
---@return string
function colors.rgb(r, g, b)
    return string.format("#%02x%02x%02x", math.floor(r), math.floor(g), math.floor(b))
end

---Format RGBA to hex color string
---@param r integer
---@param g integer
---@param b integer
---@param a number Between 0.0 and 1.0
---@return string
function colors.rgba(r, g, b, a)
    local alpha_byte = math.floor(math.min(1.0, math.max(0.0, a)) * 255)
    return string.format("#%02x%02x%02x%02x", math.floor(r), math.floor(g), math.floor(b), alpha_byte)
end

---Add or replace alpha transparency on a hex color
---@param hex_or_spec string
---@param a number Between 0.0 and 1.0
---@return string
function colors.alpha(hex_or_spec, a)
    local hex = colors.hex(tostring(hex_or_spec))
    local clean = string.gsub(hex, "^#", "")
    if #clean == 6 then
        local alpha_byte = math.floor(math.min(1.0, math.max(0.0, a)) * 255)
        return string.format("#%s%02x", clean, alpha_byte)
    elseif #clean == 8 then
        local alpha_byte = math.floor(math.min(1.0, math.max(0.0, a)) * 255)
        return string.format("#%s%02x", string.sub(clean, 1, 6), alpha_byte)
    end
    return hex
end


return colors
