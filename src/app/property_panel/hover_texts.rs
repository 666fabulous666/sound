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
pub const DETERMINISTIC_INCLUSION_TEXT: &str = concat!(
    "Rules that deterministically place beats.\n",
    "\n",
    "Choose inclusion generators: any beat whose\n",
    "time unit is a multiple of one of these\n",
    "values will be included.\n",
    "\n",
    "Generators ≤ 1 are ignored; use values > 1.",
);
pub const RANDOM_EXCLUSION_TEXT: &str = concat!(
    "Rules that randomly skip beats.\n",
    "\n",
    "A set of n exclusion generators is picked\n",
    "randomly from [2, N+1].\n",
    "\n",
    "Any beat whose time unit shifted forward\n",
    "by 1 is a multiple of one of these\n",
    "values will be excluded, ensuring the\n",
    "first beat is never excluded.\n",
    "\n",
    "(Generator 1 is not allowed,\n",
    "as it would exclude every beat.)"
);
pub const DETERMINISTIC_EXCLUSION_TEXT: &str = concat!(
    "Rules that deterministically skip beats.\n",
    "\n",
    "Choose exclusion generators: any beat whose\n",
    "time unit shifted forward by 1 is\n",
    "a multiple of one of these\n",
    "values will be excluded.\n",
    "\n",
    "Beats are tested with their time unit\n",
    "shifted forward by 1, ensuring\n",
    "the first beat is never excluded.\n",
    "\n",
    "Generators ≤ 1 are ignored; use values > 1."
);
pub const GROOVE_OFFSET_TEXT: &str = concat!(
    "Shifts the rhythmic grid used to place notes.\n",
    "\n",
    "It offsets the index of the\n",
    "time quanta tested for divisibility.\n",
    "\n",
    "This changes where note onsets are more likely\n",
    "to occur, creating an off-beat feel.\n",
    "\n",
    "Expressed in the unit of the time quantum.",
);
pub const LOOP_LENGTH_TEXT: &str = concat!(
    "Length of the loop for this sequence.\n",
    "\n",
    "When the end is reached, playback jumps\n",
    "back to zero immediately, independent of\n",
    "the loop lengths of other sequences."
);
pub const REPEAT_TEXT: &str = "How many times the sequence will be repeated.";
pub const TOLERENCE_TEXT: &str = concat!(
    "Tolerance defines how much to look\n",
    "before the note starts and after it ends.\n",
    "\n",
    "Use this to follow notes across their edges\n",
    "while generating new notes.\n",
    "Negative values are allowed.\n",
    "\n",
    "(See generating logics for more details)",
);
pub const OCTAVE_TEXT: &str = "Base octave where notes of this sequence are placed.";
pub const VARIATION_STEPS_TEXT: &str = concat!(
    "Maximum number of random variations to apply.\n",
    "\n",
    "The note is chosen from visible ones\n",
    "(based on tolerance),\n",
    "then shifted step by step using\n",
    "the allowed intervals.\n",
    "\n",
    "Higher values allow more chained shifts."
);
pub const VARIATION_INTERVALS_TEXT: &str = concat!(
    "The set of semitone intervals used for variation.\n",
    "\n",
    "Each step shifts the note by one of these values.\n",
    "Multiple steps can combine, wrapping around octaves\n",
    "(12 semitones)."
);
pub const SHUFFLE_TEXT: &str = concat!(
    "Generate notes from the sequence in a\n",
    "random order, affecting which notes follow\n",
    "one another.\n",
    "\n",
    "A note may only follow a previously\n",
    "generated other one (see tolerance for\n",
    "more settings about this point.",
);
