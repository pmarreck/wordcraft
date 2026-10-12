# Mobile editing

Phones default to Web Layout: text reflows to the available width at readable 100% zoom. Print Layout remains available to check pagination and initially fits the page to the screen. Rotating or resizing preserves an explicit view choice. Reflow changes the view, never the document's page settings or print/export layout.

The touch contract is:

- Drag one finger to scroll. Swiping preserves the caret, selection, document and objects.
- Tap to place the caret and focus text input. Tap, then immediately drag from nearby to select text. The second contact must start within 350 ms and 28 screen points of the completed tap; movement beyond 8 points starts selection.
- Use two fingers to pan and pinch to zoom about their midpoint. Adding a finger cancels single-finger selection handling until every finger has lifted.
- Cancellation or lost focus clears gesture state. A canceled gesture cannot become an edit later.

Compact controls keep file operations, undo/redo, fonts, common formatting and both views available. Other commands remain reachable through command search. Touch input is handled separately from the synthetic mouse events browsers also emit, including eframe's immediate pointer callbacks before the real touch events. UI pointer callbacks retain browser user activation for file dialogs/downloads; completed document taps update text focus during the touch callback. Mouse selection and wheel/trackpad behavior retain their existing paths.

Regression tests send timestamped egui touch events with the accompanying browser-style pointer events. They compare document content and selection before and after navigation, check the exact text replaced by a selection gesture and verify midpoint anchoring and caret visibility. Headless Chromium additionally exercises the built WASM application through WebDriver touch actions at phone/tablet sizes and compares every exported paragraph against an imported text file. These checks run in `./test` and Nix CI. Physical iOS/Android keyboard, accessibility and stylus validation require device testing.
