pub const VOICE_LAYERS_TEXT: &str = concat!(
    "Each layer adds one detuned voice\n",
    "above and below f₀.\n",
    "Voices total = 1 + 2 × (layers − 1).",
);

pub const DETUNE_TEXT: &str = concat!(
    "Detune amount between voices around f₀.\n",
    "\n",
    "Use very small values for slow beating;\n",
    "increase for a wider chorus.",
);
pub const DETUNE_SHIFT_TEXT: &str = "Shift voices frequencies asymmetrically to avoid beatings.";

pub const DETUNE_TIME_DEP_TEXT: &str = concat!(
    "Modulates Δf over time.\n",
    " > 0 : Δf increases over time.\n",
    " < 0 : Δf decreases over time.\n",
    " = 0 : static detune."
);
pub const DETUNE_WEIGHTING_TEXT: &str = concat!(
    "Sets how much outer voices contribute relative to the center.\n",
    " • |value| > 1 -> outer voices amplified\n",
    " • |value| = 1 -> constant voice levels\n",
    " • |value| < 1 -> outer voices attenuated\n",
    " •  value < 0  -> outer voices inverted in phase"
);
pub const SYM_DETUNE_TEXT: &str = "Even (symmetric) weighting across ±Δf";
pub const ASYM_DETUNE_TEXT: &str = "Odd (asymmetric) weighting across ±Δf.";
pub const UNISSON_DETUNE_TEXT: &str = concat!(
    "Adds multiple voices detuned\n",
    "around the main frequency f₀\n",
    "to create width and motion.",
);
pub const POW_FACT_EVOL_TEXT: &str = "Increase/Decrease over time.";
pub const POW_FACT_TEXT: &str = concat!(
    "Produces distortion or metallic timbre\n",
    "\n",
    " • |value| = 0 -> square wave\n",
    " • |value| < 1 -> distortion\n",
    " • |value| = 1 -> unchanged wave\n",
    " • |value| > 1 -> metallic",
);
pub const TIME_QUANTUM_TEXT: &str = concat!(
    "Duration of the base time unit for beats.\n",
    "\n",
    "Rhythm inclusions and exclusions are tested\n",
    "for divisibility against this quantum."
);
pub const RANDOM_INCLUSION_TEXT: &str = concat!(
    "Rules that randomly place beats.\n",
    "\n",
    "A set of n inclusion generators is picked\n",
    "randomly from [1, N].\n",
    "\n",
    "Any beat whose time unit is a multiple of\n",
    "one of these values will be included."
);
