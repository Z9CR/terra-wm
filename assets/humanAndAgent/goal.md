# terra-wm
## file usage
1. mark a `[yyyy/mm/dd]-[done]` suffix when a request is finished,
   like `0. call smithay to render wl-compositor    [2026/11/4]-[done]`
2. if you're a agent, when a request is diffcult to understand, 
   ask the user(just ask them, is premium-free) then make the note(ask the user beforehand)

## major feats(sorted as importance):
0. call smithay to render wl-compositor    [2026/8/17]-[done]
1. Window-Stacking in one screen           [2026/8/17]-[done]
2. Dynamic-Tiling in one screen
3. multi-layers overlay
4. switch two layers
5. infinity-layer
6. infinity-layer's translation

## sub-requests:
*(we should imple these feats while impling feats above)*
1. Server-Side Decoration
2. Command
    *like `S+c` to quit the focused window,*
    *`Three fingers slide down` to minimalize all the windows*
    1. single or combo keyboard command
    2. Mouse Gesture command
    3. Touchpad Gusture command
3. will update in future

## proper nouns
* `Tile(Tiling, Tiled)` is the same as hyprland/niri's tiling window
* `Stacking` is the same as labwc's stacking window
* `layer` terra-wm's window can be stacked or tiled on a layer, and then  
   the layer will be stacked overlay to show all the windows(whether tiled or stacked) in the physical monitor(s).
   a layer should have following properties(user editable(public interface)), actually you can add other porps for inner-system use(private interface)):
      - `window_layout_type`: `tiling` or `stacked`(in simple, hyprland style or labwc style)
      - `theme`: theme of current layer(just its literal meaning)

## detail
* `Dynamic-Tiling in one screen`, the feature aims to imple the tiling windows feat in the grid-layout. 
   Beause of `infinity-screen` and `multi-layers overlay`, 
   each tiled window's max size is the size of the smallest monitor (the layer's size is infinity 
   and user can transfer the layer and windows on it will be moved with the layer to 
   keep the relative position between them, so we need to divide a layer to pieces for tiled windows)