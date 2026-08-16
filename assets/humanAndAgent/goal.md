# terra-wm
## file usage
1. mark a `[yyyy/mm/dd]-[done]` suffix when a request is finished,
   like `0. call smithay to render wl-compositor    [2026/11/4]-[done]`
2. if you're a agent, when a request is diffcult to understand, 
   ask the user(just ask them, is premium-free) then make the note(ask the user beforehand)

## terra-wm should imple these feats(sorted as importance):
0. call smithay to render wl-compositor
1. Window-Stacking in one screen
2. Dynamic-Tiling in one screen
3. multi-layers overlay
4. switch two layers
5. infinity-screen
6. infinity-screen's translation

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