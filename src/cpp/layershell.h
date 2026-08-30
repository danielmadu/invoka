#pragma once

// Attaches the launcher window to the wlr layer shell when the build was
// made with the `layer-shell` feature and the running compositor supports
// it. Returns the number of windows attached (0 = plain window fallback).
int invoka_layershell_setup();
