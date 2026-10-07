---@meta
-- GPUI-CE LuaJIT DSL Type Definitions for LuaLS / EmmyLua

---@alias Color string | { [1]: integer, [2]: integer, [3]: integer, [4]?: number } | { r: integer, g: integer, b: integer, a?: number }
---@alias Length number | string | "full" | "auto"
---@alias DisplayMode "block" | "flex" | "grid" | "none" | "hidden"
---@alias FlexAlign "start" | "center" | "end" | "baseline" | "stretch"
---@alias FlexJustify "start" | "center" | "end" | "between" | "around" | "evenly"
---@alias ContentAlign "center" | "start" | "end" | "between" | "around" | "evenly" | "stretch"
---@alias CursorType "pointer" | "default" | "arrow" | "text" | "ibeam" | "move" | "closed_hand" | "grab" | "open_hand" | "not_allowed" | "crosshair" | "context_menu"
---@alias ShadowLevel "sm" | "md" | "lg"
---@alias ButtonVariant "primary" | "secondary" | "ghost" | "danger" | "outline"

---@class DivProps
---@field id? string Unique identifier for element
---@field display? DisplayMode Display mode ("block" | "flex" | "grid" | "none")
---@field block? boolean Set display block
---@field hidden? boolean Set display none
---@field position? "relative" | "absolute" Positioning strategy
---@field relative? boolean Set position relative
---@field absolute? boolean Set position absolute
---@field top? number | string Top offset in pixels or percentage
---@field bottom? number | string Bottom offset in pixels or percentage
---@field left? number | string Left offset in pixels or percentage
---@field right? number | string Right offset in pixels or percentage
---@field inset? number | string Inset on all 4 sides
---@field inset_0? boolean Inset 0 on all 4 sides
---@field inset_x? number | string Horizontal inset (left and right)
---@field inset_y? number | string Vertical inset (top and bottom)
---@field visible? boolean Set visibility to visible
---@field invisible? boolean Set visibility to hidden
---@field visibility? "visible" | "hidden" Element visibility
---@field flex? boolean Display flex with row direction
---@field flex_col? boolean Display flex with column direction
---@field flex_row? boolean Display flex with row direction
---@field flex_wrap? boolean Enable flex wrap
---@field flex_grow? number Flex grow factor (default 1.0)
---@field flex_1? boolean Grow and shrink to fill available space
---@field flex_auto? boolean Grow and shrink taking into account initial size
---@field flex_none? boolean Prevent flex item from growing or shrinking
---@field flex_shrink? number Flex shrink factor
---@field items_center? boolean Align items center
---@field items_start? boolean Align items start
---@field items_end? boolean Align items end
---@field items_baseline? boolean Align items along baseline
---@field items_stretch? boolean Stretch items to fill cross axis
---@field self_start? boolean Align self start
---@field self_center? boolean Align self center
---@field self_end? boolean Align self end
---@field self_stretch? boolean Align self stretch
---@field justify_center? boolean Justify content center
---@field justify_between? boolean Justify content space-between
---@field justify_start? boolean Justify content start
---@field justify_end? boolean Justify content end
---@field justify_around? boolean Justify content space-around
---@field justify_evenly? boolean Justify content space-evenly
---@field content_center? boolean Pack content center
---@field content_start? boolean Pack content start
---@field content_end? boolean Pack content end
---@field content_between? boolean Pack content space-between
---@field content_around? boolean Pack content space-around
---@field content_evenly? boolean Pack content space-evenly
---@field content_stretch? boolean Allow content to stretch
---@field gap? number Gap between children in pixels
---@field gap_x? number Horizontal gap in pixels
---@field gap_y? number Vertical gap in pixels
---@field aspect_ratio? number Aspect ratio (width / height)
---@field aspect_square? boolean Aspect ratio 1:1
---@field p? Length Padding all sides
---@field px? Length Horizontal padding
---@field py? Length Vertical padding
---@field pt? Length Top padding
---@field pr? Length Right padding
---@field pb? Length Bottom padding
---@field pl? Length Left padding
---@field m? Length Margin all sides
---@field mx? Length Horizontal margin
---@field my? Length Vertical margin
---@field mt? Length Top margin
---@field mr? Length Right margin
---@field mb? Length Bottom margin
---@field ml? Length Left margin
---@field w? Length Width
---@field w_full? boolean Width 100%
---@field h? Length Height
---@field h_full? boolean Height 100%
---@field min_w? Length Minimum width
---@field min_h? Length Minimum height
---@field max_w? Length Maximum width
---@field max_h? Length Maximum height
---@field bg? Color Background color
---@field text_color? Color Cascading text color
---@field text_bg? Color Cascading text background color
---@field rounded? number Corner radius in pixels
---@field corner_smoothing? number Superellipse smoothing between 0.0 and 1.0
---@field rounded_smoothing_ios? boolean Apply iOS-style 0.6 squircle smoothing
---@field border? number Border width in pixels
---@field border_color? Color Border color
---@field border_dashed? boolean Dashed border style
---@field border_dashed_length? number Length of each border dash
---@field border_dashed_gap? number Gap between border dashes
---@field shadow? ShadowLevel Box shadow level
---@field shadow_sm? boolean Small shadow
---@field shadow_md? boolean Medium shadow
---@field shadow_lg? boolean Large shadow
---@field opacity? number Element opacity (0.0 - 1.0)
---@field blur? number Content blur filter radius
---@field backdrop_blur? number Frosted glass backdrop blur radius
---@field overflow_hidden? boolean Clip overflowing content
---@field overflow_scroll? boolean Enable scrollable overflow
---@field overflow_fade? number | { top?: number, bottom?: number, left?: number, right?: number } Fade distance on all or specific edges
---@field overflow_fade_x? number Gradient fade distance for horizontal overflow (left & right)
---@field overflow_fade_y? number Gradient fade distance for vertical overflow (top & bottom)
---@field overflow_fade_top? number Gradient fade distance for top edge
---@field overflow_fade_bottom? number Gradient fade distance for bottom edge
---@field overflow_fade_left? number Gradient fade distance for left edge
---@field overflow_fade_right? number Gradient fade distance for right edge
---@field overflow_fade_t? number Shorthand for overflow_fade_top
---@field overflow_fade_b? number Shorthand for overflow_fade_bottom
---@field overflow_fade_l? number Shorthand for overflow_fade_left
---@field overflow_fade_r? number Shorthand for overflow_fade_right
---@field scrollbar_width? number Space reserved for scrollbar
---@field grid? boolean Enable CSS grid layout
---@field grid_cols? integer Number of grid columns
---@field grid_rows? integer Number of grid rows
---@field col_span? integer Number of columns to span
---@field col_span_full? boolean Span all grid columns
---@field row_span? integer Number of rows to span
---@field row_span_full? boolean Span all grid rows
---@field col_start? integer Grid column start line
---@field col_end? integer Grid column end line
---@field row_start? integer Grid row start line
---@field row_end? integer Grid row end line
---@field text_left? boolean Align text left
---@field text_center? boolean Align text center
---@field text_right? boolean Align text right
---@field letter_spacing? number Letter spacing in pixels
---@field truncate? boolean Prevent wrapping and truncate with ellipsis
---@field line_clamp? integer Max lines before truncating
---@field underline? boolean Text underline
---@field line_through? boolean Strikethrough line
---@field font_family? string Font family name
---@field transition? integer Transition duration in ms for all properties
---@field transitions? TransitionsConfig Detailed per-property transition config
---@field animation? AnimationConfig Timed animation configuration
---@field cursor? CursorType Mouse cursor on hover
---@field cursor_pointer? boolean Show pointer cursor on hover
---@field on_click? fun(event?: any) Click event listener
---@field on_hover? fun(hovered: boolean) Hover state listener
---@field on_drop? fun(paths: string[]) Drag-and-drop listener for external files or data
---@field stop_propagation? boolean Stop event bubbling to parent elements
---@field focusable? boolean Allow element to receive keyboard focus
---@field children? (LuaElementBuilder | string | table)[] Child elements

---@class TransitionsConfig
---@field all? integer Duration in milliseconds for all animatable properties
---@field opacity? integer Opacity transition duration in ms
---@field bg? integer Background color transition duration in ms
---@field background? integer Background color transition duration in ms
---@field w? integer Width transition duration in ms
---@field width? integer Width transition duration in ms
---@field h? integer Height transition duration in ms
---@field height? integer Height transition duration in ms
---@field rounded? integer Corner radius transition duration in ms
---@field corner_radius? integer Corner radius transition duration in ms
---@field flex_grow? integer Flex grow transition duration in ms

---@class AnimationConfig
---@field duration? integer Duration in milliseconds (default 1000)
---@field repeat? boolean Whether to loop forever (default true)
---@field easing? "linear" | "ease_in" | "ease_out" | "ease_in_out"

---@class SvgProps: DivProps
---@field path? string Path to external SVG file
---@field [1]? string Shorthand path
---@field data? string Raw SVG XML data string

---@class ImgProps: DivProps
---@field src? string Image file path or URL
---@field [1]? string Shorthand src
---@field fit? "contain" | "cover" | "fill" | "none" | "scale_down"

---@class CanvasBounds
---@field x number Origin X in pixels
---@field y number Origin Y in pixels
---@field w number Width in pixels
---@field h number Height in pixels

---@class CanvasProps: DivProps
---@field paint? fun(bounds: CanvasBounds)

---@class VideoProps: DivProps
---@field src? string | LuaCameraCapture | LuaRemoteTrack | LuaLocalTrack Video file path or stream source
---@field track? LuaRemoteTrack | LuaLocalTrack Remote or local WebRTC track
---@field camera? LuaCameraCapture Camera hardware capture handle
---@field stream? string | any Video stream source
---@field [1]? string | LuaCameraCapture | LuaRemoteTrack | LuaLocalTrack Shorthand src
---@field fit? "contain" | "cover" | "fill" | "none" | "scale_down"
---@field autoplay? boolean Start playback automatically (default true)
---@field loop? boolean Loop playback when ended (default false)
---@field muted? boolean Mute audio output (default false)
---@field volume? number Playback volume (0.0 to 1.0, default 1.0)
---@class TextProps
---@field content? string Text content
---@field size? number Font size in pixels
---@field color? Color Text color
---@field bold? boolean Bold font weight
---@field italic? boolean Italic font style
---@field underline? boolean Text underline
---@field line_through? boolean Strikethrough line
---@field line_height? number Line height in pixels
---@field font_family? string Font family name
---@field letter_spacing? number Letter spacing in pixels
---@field truncate? boolean Prevent wrapping and truncate with ellipsis
---@field line_clamp? integer Max lines before truncating
---@class LuaElementBuilder
local LuaElementBuilder = {}

---Assign an element ID
---@param id string
---@return LuaElementBuilder
function LuaElementBuilder:id(id) end

---Set layout display block
---@return LuaElementBuilder
function LuaElementBuilder:block() end

---Hide element (display none)
---@return LuaElementBuilder
function LuaElementBuilder:hidden() end


---Set element visibility to visible
---@return LuaElementBuilder
function LuaElementBuilder:visible() end

---Set element visibility to hidden
---@return LuaElementBuilder
function LuaElementBuilder:invisible() end

---Set element position to relative
---@return LuaElementBuilder
function LuaElementBuilder:relative() end

---Set element position to absolute
---@return LuaElementBuilder
function LuaElementBuilder:absolute() end

---Set top edge offset
---@param val number | string Pixels or percentage
---@return LuaElementBuilder
function LuaElementBuilder:top(val) end

---Set bottom edge offset
---@param val number | string Pixels or percentage
---@return LuaElementBuilder
function LuaElementBuilder:bottom(val) end

---Set left edge offset
---@param val number | string Pixels or percentage
---@return LuaElementBuilder
function LuaElementBuilder:left(val) end

---Set right edge offset
---@param val number | string Pixels or percentage
---@return LuaElementBuilder
function LuaElementBuilder:right(val) end

---Set inset on all 4 sides
---@param val number | string Pixels or percentage
---@return LuaElementBuilder
function LuaElementBuilder:inset(val) end

---Set inset 0 on all 4 sides
---@return LuaElementBuilder
function LuaElementBuilder:inset_0() end

---Set horizontal inset (left and right)
---@param val number | string Pixels or percentage
---@return LuaElementBuilder
function LuaElementBuilder:inset_x(val) end

---Set vertical inset (top and bottom)
---@param val number | string Pixels or percentage
---@return LuaElementBuilder
function LuaElementBuilder:inset_y(val) end
---Set layout display flex (row)
---@return LuaElementBuilder
function LuaElementBuilder:flex() end

---Set layout display flex column
---@return LuaElementBuilder
function LuaElementBuilder:flex_col() end

---Set layout display flex row
---@return LuaElementBuilder
function LuaElementBuilder:flex_row() end

---Set flex wrap
---@param wrap? boolean Default true
---@return LuaElementBuilder
function LuaElementBuilder:flex_wrap(wrap) end

---Set flex grow factor
---@param grow? number Default 1.0
---@return LuaElementBuilder
function LuaElementBuilder:flex_grow(grow) end

---Allow flex item to grow and shrink to fill available space
---@return LuaElementBuilder
function LuaElementBuilder:flex_1() end

---Allow flex item to grow and shrink taking initial size into account
---@return LuaElementBuilder
function LuaElementBuilder:flex_auto() end

---Prevent flex item from growing or shrinking
---@return LuaElementBuilder
function LuaElementBuilder:flex_none() end

---Set flex shrink factor
---@param shrink? number Default 1.0
---@return LuaElementBuilder
function LuaElementBuilder:flex_shrink(shrink) end

---Align items center along cross axis
---@return LuaElementBuilder
function LuaElementBuilder:items_center() end

---Align items start along cross axis
---@return LuaElementBuilder
function LuaElementBuilder:items_start() end

---Align items end along cross axis
---@return LuaElementBuilder
function LuaElementBuilder:items_end() end

---Align items along baseline
---@return LuaElementBuilder
function LuaElementBuilder:items_baseline() end

---Stretch items to fill cross axis
---@return LuaElementBuilder
function LuaElementBuilder:items_stretch() end

---Align self against start of cross axis
---@return LuaElementBuilder
function LuaElementBuilder:self_start() end

---Align self along center of cross axis
---@return LuaElementBuilder
function LuaElementBuilder:self_center() end

---Align self against end of cross axis
---@return LuaElementBuilder
function LuaElementBuilder:self_end() end

---Stretch self along cross axis
---@return LuaElementBuilder
function LuaElementBuilder:self_stretch() end

---Justify content center along main axis
---@return LuaElementBuilder
function LuaElementBuilder:justify_center() end

---Justify content space-between along main axis
---@return LuaElementBuilder
function LuaElementBuilder:justify_between() end

---Justify content start along main axis
---@return LuaElementBuilder
function LuaElementBuilder:justify_start() end

---Justify content end along main axis
---@return LuaElementBuilder
function LuaElementBuilder:justify_end() end

---Justify content space-around along main axis
---@return LuaElementBuilder
function LuaElementBuilder:justify_around() end

---Justify content space-evenly along main axis
---@return LuaElementBuilder
function LuaElementBuilder:justify_evenly() end

---Pack content items in center of cross axis
---@return LuaElementBuilder
function LuaElementBuilder:content_center() end

---Pack content items against start of cross axis
---@return LuaElementBuilder
function LuaElementBuilder:content_start() end

---Pack content items against end of cross axis
---@return LuaElementBuilder
function LuaElementBuilder:content_end() end

---Pack content items with equal space between each item
---@return LuaElementBuilder
function LuaElementBuilder:content_between() end

---Pack content items with equal space on each side
---@return LuaElementBuilder
function LuaElementBuilder:content_around() end

---Pack content items with equal space around each item
---@return LuaElementBuilder
function LuaElementBuilder:content_evenly() end

---Allow content items to stretch along cross axis
---@return LuaElementBuilder
function LuaElementBuilder:content_stretch() end

---Set gap between children
---@param n number Pixels
---@return LuaElementBuilder
function LuaElementBuilder:gap(n) end

---Set horizontal gap between children
---@param n number Pixels
---@return LuaElementBuilder
function LuaElementBuilder:gap_x(n) end

---Set vertical gap between children
---@param n number Pixels
---@return LuaElementBuilder
function LuaElementBuilder:gap_y(n) end

---Set aspect ratio (width / height)
---@param ratio number
---@return LuaElementBuilder
function LuaElementBuilder:aspect_ratio(ratio) end

---Set square aspect ratio (1:1)
---@return LuaElementBuilder
function LuaElementBuilder:aspect_square() end

---Set padding on all sides
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:p(val) end

---Set horizontal padding
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:px(val) end

---Set vertical padding
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:py(val) end

---Set top padding
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:pt(val) end

---Set right padding
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:pr(val) end

---Set bottom padding
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:pb(val) end

---Set left padding
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:pl(val) end

---Set margin on all sides
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:m(val) end

---Set horizontal margin
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:mx(val) end

---Set vertical margin
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:my(val) end

---Set width
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:w(val) end

---Set width to 100%
---@return LuaElementBuilder
function LuaElementBuilder:w_full() end

---Set height
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:h(val) end

---Set height to 100%
---@return LuaElementBuilder
function LuaElementBuilder:h_full() end

---Set width and height
---@param w Length
---@param h Length
---@return LuaElementBuilder
function LuaElementBuilder:size(w, h) end

---Set minimum width
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:min_w(val) end

---Set minimum height
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:min_h(val) end

---Set maximum width
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:max_w(val) end

---Set maximum height
---@param val Length
---@return LuaElementBuilder
function LuaElementBuilder:max_h(val) end

---Set background color
---@param val Color
---@return LuaElementBuilder
function LuaElementBuilder:bg(val) end

---Set text color
---@param val Color
---@return LuaElementBuilder
function LuaElementBuilder:text_color(val) end

---Set text background color
---@param val Color
---@return LuaElementBuilder
function LuaElementBuilder:text_bg(val) end

---Set corner radius
---@param r number Pixels
---@return LuaElementBuilder
function LuaElementBuilder:rounded(r) end

---Set corner smoothing (0.0 circular to 1.0 maximum squircle)
---@param amount number
---@return LuaElementBuilder
function LuaElementBuilder:corner_smoothing(amount) end

---Set iOS-style 0.6 corner smoothing
---@return LuaElementBuilder
function LuaElementBuilder:rounded_smoothing_ios() end

---Set border width
---@param b number Pixels
---@return LuaElementBuilder
function LuaElementBuilder:border(b) end

---Set border color
---@param val Color
---@return LuaElementBuilder
function LuaElementBuilder:border_color(val) end

---Set dashed border style
---@return LuaElementBuilder
function LuaElementBuilder:border_dashed() end

---Set border dash length
---@param length number
---@return LuaElementBuilder
function LuaElementBuilder:border_dashed_length(length) end

---Set border dash gap
---@param gap number
---@return LuaElementBuilder
function LuaElementBuilder:border_dashed_gap(gap) end

---Apply small box shadow
---@return LuaElementBuilder
function LuaElementBuilder:shadow_sm() end

---Apply medium box shadow
---@return LuaElementBuilder
function LuaElementBuilder:shadow_md() end

---Apply large box shadow
---@return LuaElementBuilder
function LuaElementBuilder:shadow_lg() end

---Set element opacity
---@param op number Between 0.0 and 1.0
---@return LuaElementBuilder
function LuaElementBuilder:opacity(op) end

---Blur element's own content (like CSS filter: blur)
---@param radius number Blur radius in pixels
---@return LuaElementBuilder
function LuaElementBuilder:blur(radius) end

---Blur content behind element (frosted glass, like CSS backdrop-filter: blur)
---@param radius number Blur radius in pixels
---@return LuaElementBuilder
function LuaElementBuilder:backdrop_blur(radius) end

---Hide overflowing content
---@return LuaElementBuilder
function LuaElementBuilder:overflow_hidden() end

---Enable scrollable overflow
---@return LuaElementBuilder
function LuaElementBuilder:overflow_scroll() end

---Fade horizontal overflow edges to transparent
---@param fade number Distance in pixels
---Fade overflow edges to transparent
---@param val number | { top?: number, bottom?: number, left?: number, right?: number }
---@return LuaElementBuilder
function LuaElementBuilder:overflow_fade(val) end

---Fade horizontal overflow edges to transparent
---@param fade number Distance in pixels
---@return LuaElementBuilder
function LuaElementBuilder:overflow_fade_x(fade) end

---Fade vertical overflow edges to transparent
---@param fade number Distance in pixels
---@return LuaElementBuilder
function LuaElementBuilder:overflow_fade_y(fade) end

---Fade top edge to transparent
---@param fade number Distance in pixels
---@return LuaElementBuilder
function LuaElementBuilder:overflow_fade_top(fade) end

---Fade top edge to transparent (shorthand)
---@param fade number Distance in pixels
---@return LuaElementBuilder
function LuaElementBuilder:overflow_fade_t(fade) end

---Fade bottom edge to transparent
---@param fade number Distance in pixels
---@return LuaElementBuilder
function LuaElementBuilder:overflow_fade_bottom(fade) end

---Fade bottom edge to transparent (shorthand)
---@param fade number Distance in pixels
---@return LuaElementBuilder
function LuaElementBuilder:overflow_fade_b(fade) end

---Fade left edge to transparent
---@param fade number Distance in pixels
---@return LuaElementBuilder
function LuaElementBuilder:overflow_fade_left(fade) end

---Fade left edge to transparent (shorthand)
---@param fade number Distance in pixels
---@return LuaElementBuilder
function LuaElementBuilder:overflow_fade_l(fade) end

---Fade right edge to transparent
---@param fade number Distance in pixels
---@return LuaElementBuilder
function LuaElementBuilder:overflow_fade_right(fade) end

---Fade right edge to transparent (shorthand)
---@param fade number Distance in pixels
---@return LuaElementBuilder
function LuaElementBuilder:overflow_fade_r(fade) end
---Reserve space for scrollbar
---@param width number Pixels
---@return LuaElementBuilder
function LuaElementBuilder:scrollbar_width(width) end

---Enable CSS grid layout
---@return LuaElementBuilder
function LuaElementBuilder:grid() end

---Set number of grid columns
---@param cols integer
---@return LuaElementBuilder
function LuaElementBuilder:grid_cols(cols) end

---Set number of grid rows
---@param rows integer
---@return LuaElementBuilder
function LuaElementBuilder:grid_rows(rows) end

---Set number of grid columns to span
---@param span integer
---@return LuaElementBuilder
function LuaElementBuilder:col_span(span) end

---Span all grid columns
---@return LuaElementBuilder
function LuaElementBuilder:col_span_full() end

---Set number of grid rows to span
---@param span integer
---@return LuaElementBuilder
function LuaElementBuilder:row_span(span) end

---Span all grid rows
---@return LuaElementBuilder
function LuaElementBuilder:row_span_full() end

---Set grid column start line
---@param start integer
---@return LuaElementBuilder
function LuaElementBuilder:col_start(start) end

---Set grid column end line
---@param end_line integer
---@return LuaElementBuilder
function LuaElementBuilder:col_end(end_line) end

---Set grid row start line
---@param start integer
---@return LuaElementBuilder
function LuaElementBuilder:row_start(start) end

---Set grid row end line
---@param end_line integer
---@return LuaElementBuilder
function LuaElementBuilder:row_end(end_line) end

---Align text to the left
---@return LuaElementBuilder
function LuaElementBuilder:text_left() end

---Align text to the center
---@return LuaElementBuilder
function LuaElementBuilder:text_center() end

---Align text to the right
---@return LuaElementBuilder
function LuaElementBuilder:text_right() end

---Prevent wrapping and truncate overflowing text with ellipsis
---@return LuaElementBuilder
function LuaElementBuilder:truncate() end

---Set max number of lines before truncating
---@param lines integer
---@return LuaElementBuilder
function LuaElementBuilder:line_clamp(lines) end

---Set letter spacing
---@param spacing number Pixels
---@return LuaElementBuilder
function LuaElementBuilder:letter_spacing(spacing) end

---Apply text underline
---@return LuaElementBuilder
function LuaElementBuilder:underline() end

---Apply strikethrough line
---@return LuaElementBuilder
function LuaElementBuilder:line_through() end

---Set font family
---@param name string
---@return LuaElementBuilder
function LuaElementBuilder:font_family(name) end

---Set mouse cursor style
---@param cursor CursorType "pointer" | "default" | "text" | "move" | "not_allowed" | "crosshair" | "context_menu"
---@return LuaElementBuilder
function LuaElementBuilder:cursor(cursor) end

---Show pointing hand cursor on hover
---@return LuaElementBuilder
function LuaElementBuilder:cursor_pointer() end

---Add a single child element
---@param val LuaElementBuilder | string | table
---@return LuaElementBuilder
function LuaElementBuilder:child(val) end

---Add multiple child elements
---@param table (LuaElementBuilder | string | table)[]
---@return LuaElementBuilder
function LuaElementBuilder:children(table) end

---Bind click event listener
---@param func fun(event?: any)
---@return LuaElementBuilder
function LuaElementBuilder:on_click(func) end

---Bind hover state change listener
---@param func fun(hovered: boolean)
---@return LuaElementBuilder
function LuaElementBuilder:on_hover(func) end

---Bind drag-and-drop listener for external files or data
---@param func fun(paths: string[])
---@return LuaElementBuilder
function LuaElementBuilder:on_drop(func) end

---Stop event propagation to parent elements (prevents drag-through in titlebars)
---@param val? boolean Default true
---@return LuaElementBuilder
function LuaElementBuilder:stop_propagation(val) end

---Make element focusable for keyboard input
---@param val? boolean Default true
---@return LuaElementBuilder
function LuaElementBuilder:focusable(val) end

---Set font size (for text elements)
---@param size number Font size in pixels
---@return LuaElementBuilder
function LuaElementBuilder:size(size) end

---Set text color (for text elements)
---@param val Color
---@return LuaElementBuilder
function LuaElementBuilder:color(val) end

---Set bold font weight (for text elements)
---@return LuaElementBuilder
function LuaElementBuilder:bold() end

---Set italic font style (for text elements)
---@return LuaElementBuilder
function LuaElementBuilder:italic() end

---Set line height (for text elements)
---@param lh number Line height in pixels
---@return LuaElementBuilder
function LuaElementBuilder:line_height(lh) end

---Set image object fit
---@param mode "contain" | "cover" | "fill" | "none" | "scale_down"
---@return LuaElementBuilder
function LuaElementBuilder:fit(mode) end

---Configure animated style transitions
---@param val integer | TransitionsConfig Duration in ms or configuration table
---@return LuaElementBuilder
function LuaElementBuilder:transition(val) end

---Apply timed animation to element
---@param duration_ms integer Duration in milliseconds
---@param _repeat? boolean Loop forever (default false)
---@param easing? "linear" | "ease_in" | "ease_out" | "ease_in_out"
---@return LuaElementBuilder
function LuaElementBuilder:animate(duration_ms, _repeat, easing) end

---Set font size explicitly
---@param size number Font size in pixels
---@return LuaElementBuilder
function LuaElementBuilder:font_size(size) end

---Start video playback
---@return LuaElementBuilder
function LuaElementBuilder:play() end

---Pause video playback
---@return LuaElementBuilder
function LuaElementBuilder:pause() end

---Set playback volume
---@param vol number Volume between 0.0 and 1.0
---@return LuaElementBuilder
function LuaElementBuilder:volume(vol) end

---Set looping playback
---@param loop_enabled? boolean Default true
---@return LuaElementBuilder
function LuaElementBuilder:loop(loop_enabled) end

---Set audio mute status
---@param muted? boolean Default true
---@return LuaElementBuilder
function LuaElementBuilder:muted(muted) end

---Set textarea rows count
---@param count integer
---@return LuaElementBuilder
function LuaElementBuilder:rows(count) end

---Set textarea maximum allowed lines before input is blocked
---@param count integer
---@return LuaElementBuilder
function LuaElementBuilder:max_rows(count) end
---@class InputComponentProps
---@field id? string Unique input element identifier
---@field name? string Form field name
---@field value? string Current controlled input value
---@field default_value? string Uncontrolled default value
---@field placeholder? string Placeholder text when empty
---@field placeholder_color? Color Placeholder text color
---@field type? "text" | "password" Input type ("text" or "password")
---@field masked? boolean Mask display text (e.g. for passwords)
---@field mask? string Mask character (default "•")
---@field pattern? string Lua regex validation pattern
---@field max_length? integer Maximum character length
---@field required? boolean Require non-empty value
---@field error_message? string Default validation error message
---@field validate? fun(val: string): boolean, string? Custom validation function
---@field autofocus? boolean Focus on initial mount
---@field disabled? boolean Disable editing and interaction
---@field read_only? boolean Allow selection and focus but disallow typing
---@field bg? Color Background color
---@field focus_bg? Color Background color when focused
---@field border? number Border thickness
---@field border_color? Color Border color
---@field focus_border_color? Color Border color when focused
---@field error_border_color? Color Border color on validation failure
---@field text_color? Color Text color
---@field cursor_color? Color Cursor bar color
---@field selection_bg? Color Selected text background color
---@field selection_color? Color Selected text color
---@field rounded? number Border radius
---@field px? number Horizontal padding
---@field py? number Vertical padding
---@field w? number | string Explicit width
---@field w_full? boolean Width 100%
---@field h? number | string Explicit height
---@field h_full? boolean Height 100%
---@field font_size? number Text font size
---@field font_family? string Custom font family
---@field tab_index? integer Focus tab order index
---@field on_change? fun(val: string) Triggered on text changes
---@field on_input? fun(val: string) Triggered on each keystroke
---@field on_focus? fun() Triggered when input gains focus
---@field on_blur? fun(val: string) Triggered when input loses focus
---@field on_submit? fun(val: string) Triggered on Enter key
---@field on_validate? fun(is_valid: boolean, error_msg?: string) Triggered on validation
---@field on_key_down? fun(key: string, event: table) Raw key down handler

---@class TextareaComponentProps: InputComponentProps
---@field rows? integer Visible text rows count (default 4)
---@field max_rows? integer Maximum allowed lines before input is blocked
---@field min_h? number Minimum container height
---@class RowComponentProps
---@field gap? number Gap between children
---@field align? FlexAlign "start" | "center" | "end"
---@field justify? FlexJustify "start" | "center" | "end" | "between"
---@field p? Length Padding
---@field w_full? boolean Width 100%
---@field children? (LuaElementBuilder | string | table)[]
---@field [integer]? LuaElementBuilder | string | table Shorthand children

---@class ColumnComponentProps
---@field gap? number Gap between children
---@field align? FlexAlign "start" | "center" | "end"
---@field p? Length Padding
---@field w_full? boolean Width 100%
---@field children? (LuaElementBuilder | string | table)[]
---@field [integer]? LuaElementBuilder | string | table Shorthand children

---@class StackComponentProps
---@field w_full? boolean Width 100%
---@field h_full? boolean Height 100%
---@field children? (LuaElementBuilder | string | table)[]
---@field [integer]? LuaElementBuilder | string | table Shorthand children

---@class ModalComponentProps
---@field is_open? boolean Visibility of modal
---@field title? string Modal title
---@field on_close? fun() Close button listener
---@field children? (LuaElementBuilder | string | table)[]
---@field [integer]? LuaElementBuilder | string | table Shorthand children

---@class DividerComponentProps
---@field vertical? boolean Orientation
---@field color? Color Divider color
---@field size? number Thickness in pixels

---@class UI
ui = {}

---Create a new div container
---@param props_or_children? DivProps | (LuaElementBuilder | string)[] | LuaElementBuilder | string
---@return LuaElementBuilder
function ui.div(props_or_children) end

---Create a text element
---@param str_or_props string | TextProps | number | boolean
---@return LuaElementBuilder
function ui.text(str_or_props) end

---Create a styled button
---@param label string | number
---@param on_click? fun(event?: any)
---@return LuaElementBuilder
function ui.button(label, on_click) end

---Create a vector SVG element
---@param path_or_props string | SvgProps External SVG file path or configuration table
---@return LuaElementBuilder
function ui.svg(path_or_props) end

---Create a raster image element
---@param src_or_props string | ImgProps File path/URL or configuration table
---@return LuaElementBuilder
function ui.img(src_or_props) end

---Create a custom 2D canvas drawing element
---@param paint_fn_or_props fun(bounds: CanvasBounds) | CanvasProps Paint function or configuration table
---@return LuaElementBuilder
function ui.canvas(paint_fn_or_props) end

---Create a hardware-accelerated video playback element powered by FFmpeg
---@param src_or_props string | LuaCameraCapture | LuaRemoteTrack | LuaLocalTrack | VideoProps File path/URL, camera capture, WebRTC track, or configuration table
---@return LuaElementBuilder
function ui.video(src_or_props) end

---Input element with editing, focus, selection, keyboard input, IME, masking and validation
---@param props? InputComponentProps
---@return LuaElementBuilder
function ui.input(props) end

---Multi-line textarea element with editing, focus, selection, line navigation, and validation
---@param props? TextareaComponentProps
---@return LuaElementBuilder
function ui.textarea(props) end

---Pre-built Row flex layout component
---@param props RowComponentProps
---@return LuaElementBuilder
function ui.row(props) end

---Pre-built Column flex layout component
---@param props ColumnComponentProps
---@return LuaElementBuilder
function ui.column(props) end

---Pre-built Stack overlay component
---@param props StackComponentProps
---@return LuaElementBuilder
function ui.stack(props) end

--------------------------------------------------------------------------------
-- Reactive System
--------------------------------------------------------------------------------

---Create a reactive signal with state preservation across hot reloads.
---@generic T
---@param initial_val T Initial value
---@param opt_key? string Optional unique key (auto-generated from file/line if omitted)
---@return (fun(new_val?: T): T) getter Callable getter (or setter when passed arg)
---@return (fun(new_val: T): nil) setter Explicit setter function
function signal(initial_val, opt_key) end

---Create a reactive effect that automatically runs and tracks signal reads.
---@param fn fun() Effect function
---@return fun() runner Re-run function
function effect(fn) end

---Create a memoized derived signal from other signals.
---@generic T
---@param fn fun(): T Computation function
---@return fun(): T getter Memoized getter
function computed(fn) end

---Wrap a table in a reactive proxy tracking reads and notifying on writes.
---@generic T: table
---@param tbl T Table to wrap
---@return T proxy Reactive proxy
function reactive(tbl) end

--------------------------------------------------------------------------------
-- Asynchronous Networking
--------------------------------------------------------------------------------

---@class HttpResponse
---@field status integer HTTP status code (e.g. 200)
---@field status_text string HTTP status text (e.g. "OK")
---@field ok boolean True if status is between 200 and 299
---@field headers table<string, string> Response headers
---@field body string Response body string
---@field json fun(self?: HttpResponse): any Parse response body as JSON table

---@class HttpRequestOptions
---@field method? string HTTP method (GET, POST, PUT, DELETE, etc.)
---@field url? string Request URL
---@field headers? table<string, string> Request headers
---@field body? string | table Request body (tables auto-encoded to JSON)
---@field timeout? integer Timeout in seconds (default 30)

---@alias HttpCallback fun(err: string|nil, resp: HttpResponse|nil)

---@class HttpModule
http = {}

---Send an asynchronous GET request
---@param url string
---@param opts_or_cb? HttpRequestOptions | HttpCallback
---@param callback? HttpCallback
function http.get(url, opts_or_cb, callback) end

---Send an asynchronous POST request
---@param url string
---@param opts_or_cb? HttpRequestOptions | HttpCallback
---@param callback? HttpCallback
function http.post(url, opts_or_cb, callback) end

---Send an asynchronous PUT request
---@param url string
---@param opts_or_cb? HttpRequestOptions | HttpCallback
---@param callback? HttpCallback
function http.put(url, opts_or_cb, callback) end

---Send an asynchronous DELETE request
---@param url string
---@param opts_or_cb? HttpRequestOptions | HttpCallback
---@param callback? HttpCallback
function http.delete(url, opts_or_cb, callback) end

---Send an asynchronous request with options
---@param options HttpRequestOptions
---@param callback? HttpCallback
function http.request(options, callback) end

---Coroutine-friendly synchronous-style fetch
---@param url string
---@param opts? HttpRequestOptions
---@return HttpResponse? response
---@return string? error
function http.fetch(url, opts) end

--------------------------------------------------------------------------------
-- Timers
--------------------------------------------------------------------------------

---@class TimerModule
timer = {}

---Execute a callback after a delay
---@param ms integer Delay in milliseconds
---@param callback fun()
---@return integer id Timer ID for timer.clear()
function timer.set_timeout(ms, callback) end

---Execute a callback repeatedly at a set interval
---@param ms integer Interval in milliseconds
---@param callback fun()
---@return integer id Timer ID for timer.clear()
function timer.set_interval(ms, callback) end

---Cancel an active timeout or interval
---@param id integer Timer ID
function timer.clear(id) end

--------------------------------------------------------------------------------
-- JSON
--------------------------------------------------------------------------------

---@class JsonModule
json = {}

---Encode a Lua value or table to a JSON string
---@param val any
---@return string json_string
function json.encode(val) end

---Decode a JSON string to a Lua table or value
---@param str string
---@return any value
function json.decode(str) end

--------------------------------------------------------------------------------
-- Filesystem
--------------------------------------------------------------------------------

---@class FsModule
fs = {}

---Read the contents of a file as a string
---@param path string
---@return string? content
---@return string? error
function fs.read(path) end

---Write data to a file (creates parent directories if needed)
---@param path string
---@param data string | number
---@return boolean success
---@return string? error
function fs.write(path, data) end

---Check if a file or directory exists
---@param path string
---@return boolean exists
function fs.exists(path) end

--------------------------------------------------------------------------------
-- Logging
--------------------------------------------------------------------------------

---@class LogModule
log = {}

---Log at DEBUG level
---@param ... any
function log.debug(...) end

---Log at INFO level
---@param ... any
function log.info(...) end

---Log at WARN level
---@param ... any
function log.warn(...) end

---Log at ERROR level
---@param ... any
function log.error(...) end

--------------------------------------------------------------------------------
-- Colors
--------------------------------------------------------------------------------

---@class ColorsModule
colors = {}

---Normalize or prepend # to hex color
---@param code string
---@return string
function colors.hex(code) end

---Format RGB to hex color string
---@param r integer
---@param g integer
---@param b integer
---@return string
function colors.rgb(r, g, b) end

---Format RGBA to hex color string
---@param r integer
---@param g integer
---@param b integer
---@param a number Between 0.0 and 1.0
---@return string
function colors.rgba(r, g, b, a) end

---Add or replace alpha transparency on a hex color
---@param hex_or_spec string
---@param a number Between 0.0 and 1.0
---@return string
function colors.alpha(hex_or_spec, a) end

--------------------------------------------------------------------------------
-- OS & Platform
--------------------------------------------------------------------------------

---@class OsClipboard
local OsClipboard = {}

---Read text from the system clipboard
---@return string? text
function OsClipboard.get() end

---Write text to the system clipboard
---@param text string
function OsClipboard.set(text) end

---Clear the system clipboard
function OsClipboard.clear() end

---@class OsWindow
local OsWindow = {}

---Set the window title
---@param title string
function OsWindow.set_title(title) end

---Minimize the window
function OsWindow.minimize() end

---Maximize / zoom the window
function OsWindow.maximize() end

---Toggle fullscreen on the window
function OsWindow.toggle_fullscreen() end

---Check if window is fullscreen
---@return boolean
function OsWindow.is_fullscreen() end

---Check if window is maximized
---@return boolean
function OsWindow.is_maximized() end

---Get window viewport size
---@return { width: number, height: number }
function OsWindow.get_size() end

---@class OsApp
local OsApp = {}

---Quit the application
function OsApp.quit() end

---@class OsModule
---@field platform "windows" | "macos" | "linux"
---@field arch string CPU architecture
---@field clipboard OsClipboard
---@field window OsWindow
---@field app OsApp
os = {}

---Open URL in system default browser
---@param url string
function os.open_url(url) end

---Open path in system file manager / explorer
---@param path string
function os.open_path(path) end

--------------------------------------------------------------------------------
-- Rust Backend Interop
--------------------------------------------------------------------------------

---@class BackendModule
---@field [string] fun(...: any): any Dynamic invocation of registered Rust backend functions
backend = {}

---Invoke a synchronous Rust backend function by name
---@param name string Registered function name
---@param ... any Arguments passed to Rust
---@return any result Value returned by Rust
function backend.call(name, ...) end

---Invoke an asynchronous Rust backend task
---@param name string Registered function name
---@param ... any Arguments followed by optional callback function(err, result)
function backend.call_async(name, ...) end

---Subscribe to an event emitted from the Rust backend
---@param event string Event name
---@param handler fun(data: any) Callback handler
function backend.on(event, handler) end

---Emit an event from the Lua frontend to Rust listeners
---@param event string Event name
---@param data any Event payload (tables auto-converted to JSON)
function backend.emit(event, data) end

--------------------------------------------------------------------------------
-- Audio Utility (FFmpeg)
--------------------------------------------------------------------------------

---@class AudioPlayerHandle
local AudioPlayerHandle = {}

---Start or resume audio playback
function AudioPlayerHandle:play() end

---Pause audio playback
function AudioPlayerHandle:pause() end

---Stop audio playback and reset position
function AudioPlayerHandle:stop() end

---Set playback volume
---@param vol number Volume between 0.0 and 1.0
function AudioPlayerHandle:volume(vol) end

---Set looping playback
---@param loop_enabled? boolean Default true
function AudioPlayerHandle:loop(loop_enabled) end

---Seek to position in seconds
---@param seconds number
function AudioPlayerHandle:seek(seconds) end

---Get audio duration in seconds
---@return number
function AudioPlayerHandle:duration() end

---Get current playback position in seconds
---@return number
function AudioPlayerHandle:position() end

---Check if audio is currently playing
---@return boolean
function AudioPlayerHandle:is_playing() end

---@class AudioModule
audio = {}

---Load an audio file into a player handle without playing
---@param path string File path or stream URL
---@return AudioPlayerHandle
function audio.load(path) end

---Load and immediately start playing an audio file
---@param path string File path or stream URL
---@return AudioPlayerHandle
function audio.play(path) end

--------------------------------------------------------------------------------
-- Media Module (Hardware Camera & Microphone Capture)
--------------------------------------------------------------------------------

---@class CameraDeviceInfo
---@field id integer Device index (0 is OS default)
---@field name string Friendly device name
---@field default boolean Whether this is the system default camera

---@class MicrophoneDeviceInfo
---@field id integer Device index (0 is OS default)
---@field name string Friendly device name
---@field default boolean Whether this is the system default microphone

---@class CameraOptions
---@field device? integer Camera device index (0 = OS default)
---@field name? string Exact device name to bind
---@field width? integer Frame width in pixels (default 1280)
---@field height? integer Frame height in pixels (default 720)
---@field fps? integer Target framerate (default 30)

---@class MicrophoneOptions
---@field device? integer Microphone device index (0 = OS default)
---@field name? string Exact device name to bind
---@field sample_rate? integer Audio sample rate in Hz (default 48000)
---@field channels? integer Channel count (1 = mono, 2 = stereo, default 2)

---@class LuaCameraCapture
local LuaCameraCapture = {}

---Get the camera stream URI (e.g. "camera://0")
---@return string
function LuaCameraCapture:src() end

---Get the camera stream URI
---@return string
function LuaCameraCapture:uri() end

---Get the friendly camera hardware name
---@return string
function LuaCameraCapture:name() end

---Get unique camera capture session ID
---@return integer
function LuaCameraCapture:id() end

---Get camera frame width
---@return integer
function LuaCameraCapture:width() end

---Get camera frame height
---@return integer
function LuaCameraCapture:height() end

---Get camera framerate
---@return integer
function LuaCameraCapture:fps() end

---Mute or unmute camera video stream
---@param muted boolean
function LuaCameraCapture:mute(muted) end

---Check if camera is currently muted
---@return boolean
function LuaCameraCapture:is_muted() end

---Stop camera capture and release device
function LuaCameraCapture:stop() end

---@class LuaMicrophoneCapture
local LuaMicrophoneCapture = {}

---Get the microphone stream URI (e.g. "microphone://0")
---@return string
function LuaMicrophoneCapture:src() end

---Get the microphone stream URI
---@return string
function LuaMicrophoneCapture:uri() end

---Get the friendly microphone hardware name
---@return string
function LuaMicrophoneCapture:name() end

---Get unique microphone capture session ID
---@return integer
function LuaMicrophoneCapture:id() end

---Mute or unmute microphone audio stream
---@param muted boolean
function LuaMicrophoneCapture:mute(muted) end

---Check if microphone is currently muted
---@return boolean
function LuaMicrophoneCapture:is_muted() end

---Stop microphone capture and release audio device
function LuaMicrophoneCapture:stop() end

---@class MediaModule
media = {}

---List all detected hardware camera devices
---@return CameraDeviceInfo[]
function media.list_cameras() end

---List all detected hardware microphone devices
---@return MicrophoneDeviceInfo[]
function media.list_microphones() end

---Open camera video stream
---@param options? CameraOptions
---@return LuaCameraCapture
function media.open_camera(options) end
---Open microphone audio stream
---@param options? MicrophoneOptions
---@return LuaMicrophoneCapture
function media.open_microphone(options) end
---Bind media stream to an element or player
---@param source string | LuaCameraCapture | LuaMicrophoneCapture
---@param target? any
---@return string Stream URI
function media.bind(source, target) end

--------------------------------------------------------------------------------
-- WebRTC Module (P2P DataChannels, Audio & Video Tracks)
--------------------------------------------------------------------------------

---@class IceServerConfig
---@field urls string | string[] STUN/TURN server URLs
---@field username? string Authentication username
---@field credential? string Authentication password/credential

---@class RtcConfiguration
---@field ice_servers? IceServerConfig[]

---@class SessionDescription
---@field type "offer" | "answer"
---@field sdp string

---@class IceCandidateInit
---@field candidate string
---@field sdpMid? string
---@field sdpMLineIndex? integer

---@class LuaDataChannel
local LuaDataChannel = {}

---Send string or JSON table through data channel
---@param data string | table
function LuaDataChannel:send(data) end

---Close the data channel
function LuaDataChannel:close() end

---Attach message callback
---@param handler fun(text: string)
function LuaDataChannel:on_message(handler) end

---Attach open callback
---@param handler fun()
function LuaDataChannel:on_open(handler) end

---Attach close callback
---@param handler fun()
function LuaDataChannel:on_close(handler) end

---@class LuaRemoteTrack
local LuaRemoteTrack = {}

---Get remote track ID
---@return string
function LuaRemoteTrack:id() end

---Get remote track media kind ("video" | "audio")
---@return "video" | "audio"
function LuaRemoteTrack:kind() end

---Get stream URI for binding into ui.video or ui.audio
---@return string
function LuaRemoteTrack:src() end

---Get stream URI
---@return string
function LuaRemoteTrack:uri() end

---Start audio or video playback for this track
---@return string Stream URI
function LuaRemoteTrack:play() end

---@class LuaLocalTrack
local LuaLocalTrack = {}

---Get local track ID
---@return string
function LuaLocalTrack:id() end

---Get local track media kind ("video" | "audio")
---@return "video" | "audio"
function LuaLocalTrack:kind() end

---Get stream URI
---@return string
function LuaLocalTrack:src() end

---Get stream URI
---@return string
function LuaLocalTrack:uri() end

---Stop streaming this track
function LuaLocalTrack:stop() end

---@class LuaPeerConnection
local LuaPeerConnection = {}

---Create SDP offer
---@param options? table
---@return Promise<SessionDescription>
function LuaPeerConnection:create_offer(options) end

---Create SDP answer
---@return Promise<SessionDescription>
function LuaPeerConnection:create_answer() end

---Set remote session description
---@param desc SessionDescription
---@return Promise<boolean>
function LuaPeerConnection:set_remote_description(desc) end

---Add ICE candidate from remote peer
---@param candidate IceCandidateInit
function LuaPeerConnection:add_ice_candidate(candidate) end

---Create new WebRTC data channel
---@param label string Channel name
---@param options? table
---@return LuaDataChannel
function LuaPeerConnection:create_data_channel(label, options) end

---Add local camera or microphone media track
---@param source LuaCameraCapture | LuaMicrophoneCapture
---@return LuaLocalTrack
function LuaPeerConnection:add_track(source) end

---Attach callback for incoming remote tracks
---@param handler fun(track: LuaRemoteTrack)
function LuaPeerConnection:on_track(handler) end

---Attach callback for local ICE candidates to signal to remote peer
---@param handler fun(candidate: IceCandidateInit)
function LuaPeerConnection:on_ice_candidate(handler) end

---Attach callback for connection state changes ("connecting" | "connected" | "disconnected" | "failed" | "closed")
---@param handler fun(state: "new" | "connecting" | "connected" | "disconnected" | "failed" | "closed")
function LuaPeerConnection:on_connection_state_change(handler) end

---Attach callback for incoming remote data channels
---@param handler fun(channel: LuaDataChannel)
function LuaPeerConnection:on_data_channel(handler) end

---Close the peer connection
function LuaPeerConnection:close() end

---@class WebRtcModule
webrtc = {}

---Create a new WebRTC PeerConnection
---@param config? RtcConfiguration
---@return LuaPeerConnection
function webrtc.create_peer_connection(config) end
