# terra-wm
## file usage
1. Mark a `[yyyy/mm/dd]-[done]` suffix when a request is finished,
   like `0. call smithay to render wl-compositor    [2026/11/4]-[done]`
2. If you're a agent, when a request is diffcult to understand, 
   ask the user(just ask them, is premium-free) then make the note(ask the user beforehand)

## major feats(sorted as importance):
0. Call smithay to render wl-compositor    [2026/8/17]-[done]
1. Window-Stacking in one screen           [2026/8/17]-[done]
2. Multi-layers overlay
3. Switch two layers
4. Infinity-layer
5. Infinity-layer's translation

## sub-requests:
*(we should imple these feats while impling feats above)*
1. Server-Side Decoration
2. Command
    *like `S+c` to quit the focused window,*
    *`Three fingers slide down` to minimalize all the windows*
    1. Single or combo keyboard command
    2. Mouse Gesture command
    3. Touchpad Gusture command
3. will update in future

## proper nouns
* `vscreen` every layer will be divided into rectangles, and the size of each rectangle is the size of the smallest physical-monitor
   - Method to detect a window is in which `vscreen`
     1. Obtain the center point (intersection of diagonals) of the window
     2. if point is on the border of the vscreen, use right-top vscreen.
        Otherwise, get current vscreen of the point
* `Stacking` is the same as labwc's stacking window
* `layer` terra-wm's window can be stacked on a layer, and then  
   the layer will be stacked overlay to show all the windows in the physical monitor(s).
   a layer should have following properties(user editable(public interface)), actually you can add other porps for inner-system use(private interface)):
      - `theme`: theme of current layer(just its literal meaning)
      - `focused`: a bool value, when `true`, window on it will accept user's input