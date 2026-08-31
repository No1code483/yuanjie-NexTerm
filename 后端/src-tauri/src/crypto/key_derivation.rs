use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::error::app_error::AppError;

const KEK_LENGTH: usize = 32;
const SALT_LENGTH: usize = 16;

pub fn generate_salt() -> Vec<u8> {
    let mut salt = vec![0u8; SALT_LENGTH];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

pub fn derive_kek(password: &str, salt: &[u8]) -> Result<[u8; KEK_LENGTH], AppError> {
    let salt_string = SaltString::encode_b64(salt)
        .map_err(|e| AppError::Crypto(format!("Salt 编码失败: {}", e)))?;

    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt_string)
        .map_err(|e| AppError::Crypto(format!("密钥派生失败: {}", e)))?;

    let hash_bytes = password_hash.hash.ok_or_else(|| AppError::Crypto("哈希结果为空".into()))?;

    let mut hasher = Sha256::new();
    hasher.update(hash_bytes.as_bytes());
    let result = hasher.finalize();

    let mut kek = [0u8; KEK_LENGTH];
    kek.copy_from_slice(&result[..KEK_LENGTH]);
    Ok(kek)
}

pub fn hash_password(password: &str, salt: &[u8]) -> Result<String, AppError> {
    let salt_string = SaltString::encode_b64(salt)
        .map_err(|e| AppError::Crypto(format!("Salt 编码失败: {}", e)))?;

    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt_string)
        .map_err(|e| AppError::Crypto(format!("密码哈希失败: {}", e)))?;

    Ok(password_hash.to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|e| AppError::Crypto(format!("密码哈希解析失败: {}", e)))?;

    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn generate_recovery_phrase(word_count: usize) -> String {
    let words = vec![
        "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd", "abuse",
        "access", "accident", "account", "accuse", "achieve", "acid", "acoustic", "acquire", "across", "act",
        "action", "actor", "actress", "actual", "adapt", "add", "addict", "address", "adjust", "admit",
        "adult", "advance", "advice", "aerobic", "affair", "afford", "afraid", "africa", "after", "again",
        "age", "agent", "agree", "ahead", "aim", "air", "airport", "aisle", "alarm", "album",
        "alcohol", "alert", "alien", "all", "alley", "allow", "almost", "alone", "alpha", "already",
        "also", "alter", "always", "amateur", "amazing", "among", "amount", "amused", "analyst", "anchor",
        "ancient", "anger", "angle", "angry", "animal", "ankle", "announce", "annual", "another", "answer",
        "antenna", "antique", "anxiety", "any", "apart", "apology", "appear", "apple", "approve", "april",
        "arch", "arctic", "area", "arena", "argue", "arm", "armed", "armor", "army", "around",
        "arrange", "arrest", "arrive", "arrow", "art", "artefact", "artist", "artwork", "ask", "aspect",
        "assault", "asset", "assist", "assume", "asthma", "athlete", "atom", "attack", "attend", "attitude",
        "attract", "auction", "audit", "august", "aunt", "author", "auto", "autumn", "average", "avocado",
        "avoid", "awake", "aware", "away", "awesome", "awful", "awkward", "axis", "baby", "bachelor",
        "bacon", "badge", "bag", "balance", "balcony", "ball", "bamboo", "banana", "banner", "bar",
        "barely", "bargain", "barrel", "base", "basic", "basket", "battle", "beach", "bean", "beauty",
        "because", "become", "beef", "before", "begin", "behave", "behind", "believe", "below", "belt",
        "bench", "benefit", "best", "betray", "better", "between", "beyond", "bicycle", "bid", "bike",
        "bind", "biology", "bird", "birth", "bitter", "black", "blade", "blame", "blanket", "blast",
        "bleak", "bless", "blind", "blood", "blossom", "blouse", "blue", "blur", "blush", "board",
        "boat", "body", "boil", "bomb", "bone", "bonus", "book", "boost", "border", "boring",
        "borrow", "boss", "bottom", "bounce", "box", "boy", "bracket", "brain", "brand", "brass",
        "brave", "bread", "breeze", "brick", "bridge", "brief", "bright", "bring", "brisk", "broccoli",
        "broken", "bronze", "broom", "brother", "brown", "brush", "bubble", "buddy", "budget", "buffalo",
        "build", "bulb", "bulk", "bullet", "bundle", "bunker", "burden", "burger", "burst", "bus",
        "business", "busy", "butter", "buyer", "buzz", "cabbage", "cabin", "cable", "cactus", "cage",
        "cake", "call", "calm", "camera", "camp", "can", "canal", "cancel", "candy", "cannon",
        "canoe", "canvas", "canyon", "capable", "capital", "captain", "car", "carbon", "card", "cargo",
        "carpet", "carry", "cart", "case", "cash", "casino", "castle", "casual", "cat", "catalog",
        "catch", "category", "cattle", "caught", "cause", "caution", "cave", "ceiling", "celery", "cement",
        "census", "century", "cereal", "certain", "chair", "chalk", "champion", "change", "chaos", "chapter",
        "charge", "chase", "chat", "cheap", "check", "cheese", "chef", "cherry", "chest", "chicken",
        "chief", "child", "chimney", "choice", "choose", "chronic", "chuckle", "chunk", "churn", "cigar",
        "cinnamon", "circle", "citizen", "city", "civil", "claim", "clap", "clarify", "claw", "clay",
        "clean", "clerk", "clever", "click", "client", "cliff", "climb", "clinic", "clip", "clock",
        "clog", "close", "cloth", "cloud", "clown", "club", "clump", "cluster", "clutch", "coach",
        "coast", "coconut", "code", "coffee", "coil", "coin", "collect", "color", "column", "combine",
        "come", "comfort", "comic", "common", "company", "concert", "conduct", "confirm", "congress", "connect",
        "consider", "control", "convince", "cook", "cool", "copper", "copy", "coral", "core", "corn",
        "correct", "cost", "cotton", "couch", "country", "couple", "course", "cousin", "cover", "coyote",
        "crack", "cradle", "craft", "cram", "crane", "crash", "crater", "crawl", "crazy", "cream",
        "credit", "creek", "crew", "cricket", "crime", "crisp", "critic", "crop", "cross", "crouch",
        "crowd", "crucial", "cruel", "cruise", "crumble", "crunch", "crush", "cry", "crystal", "cube",
        "culture", "cup", "cupboard", "curious", "current", "curtain", "curve", "cushion", "custom", "cute",
        "cycle", "dad", "damage", "damp", "dance", "danger", "daring", "dash", "daughter", "dawn",
        "day", "deal", "debate", "debris", "decade", "december", "decide", "decline", "decorate", "decrease",
        "deer", "defense", "define", "defy", "degree", "delay", "deliver", "demand", "demise", "denial",
        "dentist", "deny", "depart", "depend", "deposit", "depth", "deputy", "derive", "describe", "desert",
        "design", "desk", "despair", "destroy", "detail", "detect", "develop", "device", "devote", "diagram",
        "dial", "diamond", "diary", "dice", "diesel", "diet", "differ", "digital", "dignity", "dilemma",
        "dinner", "dinosaur", "direct", "dirt", "disagree", "discover", "disease", "dish", "dismiss", "disorder",
        "display", "distance", "divert", "divide", "divorce", "dizzy", "doctor", "document", "dog", "doll",
        "dolphin", "domain", "donate", "donkey", "donor", "door", "dose", "double", "dove", "draft",
        "dragon", "drama", "drastic", "draw", "dream", "dress", "drift", "drill", "drink", "drip",
        "drive", "drop", "drum", "dry", "duck", "dumb", "dune", "during", "dust", "dutch",
        "duty", "dwarf", "dynamic", "eager", "eagle", "early", "earn", "earth", "easily", "east",
        "easy", "echo", "ecology", "economy", "edge", "edit", "educate", "effort", "egg", "eight",
        "either", "elbow", "elder", "electric", "elegant", "element", "elephant", "elevator", "elite", "else",
        "embark", "embody", "embrace", "emerge", "emotion", "employ", "empower", "empty", "enable", "enact",
        "end", "endless", "endorse", "enemy", "energy", "enforce", "engage", "engine", "enhance", "enjoy",
        "enlist", "enough", "enrich", "enroll", "ensure", "enter", "entire", "entry", "envelope", "episode",
        "equal", "equip", "era", "erase", "erode", "erosion", "error", "erupt", "escape", "essay",
        "essence", "estate", "eternal", "ethics", "evidence", "evil", "evoke", "evolve", "exact", "example",
        "excess", "exchange", "excite", "exclude", "excuse", "execute", "exercise", "exhaust", "exhibit", "exile",
        "exist", "exit", "exotic", "expand", "expect", "expire", "explain", "expose", "express", "extend",
        "extra", "eye", "eyebrow", "fabric", "face", "faculty", "fade", "faint", "faith", "fall",
        "false", "fame", "family", "famous", "fan", "fancy", "fantasy", "farm", "fashion", "fat",
        "fatal", "father", "fatigue", "fault", "favorite", "feature", "february", "federal", "fee", "feed",
        "feel", "female", "fence", "festival", "fetch", "fever", "few", "fiber", "fiction", "field",
        "figure", "file", "film", "filter", "final", "find", "fine", "finger", "finish", "fire",
        "firm", "first", "fiscal", "fish", "fit", "fitness", "fix", "flag", "flame", "flash",
        "flat", "flavor", "flee", "flight", "flip", "float", "flock", "floor", "flower", "fluid",
        "flush", "fly", "foam", "focus", "fog", "foil", "fold", "follow", "food", "foot",
        "force", "forest", "forget", "fork", "fortune", "forum", "forward", "fossil", "foster", "found",
        "fox", "fragile", "frame", "frequent", "fresh", "friend", "fringe", "frog", "front", "frost",
        "frown", "frozen", "fruit", "fuel", "fun", "funny", "furnace", "fury", "future", "gadget",
        "gain", "galaxy", "gallery", "game", "gap", "garage", "garbage", "garden", "garlic", "garment",
        "gas", "gasp", "gate", "gather", "gauge", "gaze", "general", "genius", "genre", "gentle",
        "genuine", "gesture", "ghost", "giant", "gift", "giggle", "ginger", "giraffe", "girl", "give",
        "glad", "glance", "glare", "glass", "glide", "glimpse", "globe", "gloom", "glory", "glove",
        "glow", "glue", "goat", "goddess", "gold", "good", "goose", "gorilla", "gospel", "gossip",
        "govern", "gown", "grab", "grace", "grain", "grant", "grape", "grass", "gravity", "great",
        "green", "grid", "grief", "grit", "grocery", "group", "grow", "grunt", "guard", "guess",
        "guide", "guilt", "guitar", "gun", "gym", "habit", "hair", "half", "hammer", "hamster",
        "hand", "happy", "harbor", "hard", "harsh", "harvest", "hat", "have", "hawk", "hazard",
        "head", "health", "heart", "heavy", "hedgehog", "height", "hello", "helmet", "help", "hen",
        "hero", "hidden", "high", "hill", "hint", "hip", "hire", "history", "hobby", "hockey",
        "hold", "hole", "holiday", "hollow", "home", "honey", "hood", "hope", "horn", "horror",
        "horse", "hospital", "host", "hotel", "hour", "hover", "hub", "huge", "human", "humble",
        "humor", "hundred", "hungry", "hunt", "hurdle", "hurry", "hurt", "husband", "hybrid", "ice",
        "icon", "idea", "identify", "idle", "ignore", "ill", "illegal", "illness", "image", "imitate",
        "immense", "immune", "impact", "impose", "improve", "impulse", "inch", "include", "income", "increase",
        "index", "indicate", "indoor", "industry", "infant", "inflict", "inform", "inhale", "inherit", "initial",
        "inject", "injury", "inmate", "inner", "innocent", "input", "inquiry", "insane", "insect", "inside",
        "inspire", "install", "intact", "interest", "into", "invest", "invite", "involve", "iron", "island",
        "isolate", "issue", "item", "ivory", "jacket", "jaguar", "jar", "jazz", "jealous", "jeans",
        "jelly", "jewel", "job", "join", "joke", "journey", "joy", "judge", "juice", "jump",
        "jungle", "junior", "junk", "just", "kangaroo", "keen", "keep", "ketchup", "key", "kick",
        "kid", "kidney", "kind", "kingdom", "kiss", "kit", "kitchen", "kite", "kitten", "kiwi",
        "knee", "knife", "knock", "know", "lab", "label", "labor", "ladder", "lady", "lake",
        "lamp", "language", "laptop", "large", "later", "latin", "laugh", "laundry", "lava", "law",
        "lawn", "lawsuit", "layer", "lazy", "leader", "leaf", "learn", "leave", "lecture", "left",
        "leg", "legal", "legend", "leisure", "lemon", "lend", "length", "lens", "leopard", "lesson",
        "letter", "level", "liar", "liberty", "library", "license", "life", "lift", "light", "like",
        "limb", "limit", "link", "lion", "liquid", "list", "little", "live", "lizard", "load",
        "loan", "lobster", "local", "lock", "logic", "lonely", "long", "loop", "lottery", "loud",
        "lounge", "love", "loyal", "lucky", "luggage", "lumber", "lunar", "lunch", "luxury", "lyrics",
        "machine", "mad", "magic", "magnet", "maid", "mail", "main", "major", "make", "mammal",
        "man", "manage", "mandate", "mango", "mansion", "manual", "maple", "marble", "march", "margin",
        "marine", "market", "marriage", "mask", "mass", "master", "match", "material", "math", "matrix",
        "matter", "maximum", "maze", "meadow", "mean", "measure", "meat", "mechanic", "medal", "media",
        "melody", "melt", "member", "memory", "mention", "menu", "mercy", "merge", "merit", "merry",
        "mesh", "message", "metal", "method", "middle", "midnight", "milk", "million", "mimic", "mind",
        "minimum", "minor", "minute", "miracle", "mirror", "misery", "miss", "mistake", "mix", "mixed",
        "mixture", "mobile", "model", "modify", "mom", "moment", "monitor", "monkey", "monster", "month",
        "moon", "moral", "more", "morning", "mosquito", "mother", "motion", "motor", "mountain", "mouse",
        "move", "movie", "much", "muffin", "mule", "multiply", "muscle", "museum", "mushroom", "music",
        "must", "mutual", "myself", "mystery", "myth", "naive", "name", "napkin", "narrow", "nasty",
        "nation", "nature", "near", "neck", "need", "negative", "neglect", "neither", "nephew", "nerve",
        "nest", "net", "network", "neutral", "never", "news", "next", "nice", "night", "noble",
        "noise", "nominee", "noodle", "normal", "north", "nose", "notable", "note", "nothing", "notice",
        "novel", "now", "nuclear", "number", "nurse", "nut", "oak", "obey", "object", "oblige",
        "obscure", "observe", "obtain", "obvious", "occur", "ocean", "october", "odor", "off", "offer",
        "office", "often", "oil", "okay", "old", "olive", "olympic", "omit", "once", "one",
        "onion", "online", "only", "open", "opera", "opinion", "oppose", "option", "orange", "orbit",
        "orchard", "order", "ordinary", "organ", "orient", "original", "orphan", "ostrich", "other", "outdoor",
        "outer", "output", "outside", "oval", "oven", "over", "own", "owner", "oxygen", "oyster",
        "ozone", "pact", "paddle", "page", "pair", "palace", "palm", "panda", "panel", "panic",
        "panther", "paper", "parade", "parent", "park", "parrot", "party", "pass", "patch", "path",
        "patient", "patrol", "pattern", "pause", "pave", "payment", "peace", "peanut", "pear", "peasant",
        "pelican", "pen", "penalty", "pencil", "people", "pepper", "perfect", "permit", "person", "pet",
        "phone", "photo", "phrase", "physical", "piano", "picnic", "picture", "piece", "pig", "pigeon",
        "pill", "pilot", "pink", "pioneer", "pipe", "pistol", "pitch", "pizza", "place", "planet",
        "plastic", "plate", "play", "please", "pledge", "pluck", "plug", "plunge", "poem", "poet",
        "point", "polar", "pole", "police", "pond", "pony", "pool", "popular", "portion", "position",
        "possible", "post", "potato", "pottery", "poverty", "powder", "power", "practice", "praise", "predict",
        "prefer", "prepare", "present", "pretty", "prevent", "price", "pride", "primary", "print", "priority",
        "prison", "private", "prize", "problem", "process", "produce", "profit", "program", "project", "promote",
        "proof", "property", "prosper", "protect", "proud", "provide", "public", "pudding", "pull", "pulp",
        "pulse", "pumpkin", "punch", "pupil", "puppy", "purchase", "purity", "purpose", "purse", "push",
        "put", "puzzle", "pyramid", "quality", "quantum", "quarter", "question", "quick", "quit", "quiz",
        "quote", "rabbit", "raccoon", "race", "rack", "radar", "radio", "rail", "rain", "raise",
        "rally", "ramp", "ranch", "random", "range", "rapid", "rare", "rate", "rather", "raven",
        "raw", "razor", "ready", "real", "reason", "rebel", "rebuild", "recall", "receive", "recipe",
        "record", "recycle", "reduce", "reflect", "reform", "refuse", "region", "regret", "regular", "reject",
        "relax", "release", "relief", "rely", "remain", "remember", "remind", "remove", "render", "renew",
        "rent", "reopen", "repair", "repeat", "replace", "report", "require", "rescue", "resemble", "resist",
        "resource", "response", "result", "retire", "retreat", "return", "reunion", "reveal", "review", "reward",
        "rhythm", "rib", "ribbon", "rice", "rich", "ride", "ridge", "rifle", "right", "rigid",
        "ring", "riot", "ripple", "risk", "ritual", "rival", "river", "road", "roast", "robot",
        "robust", "rocket", "romance", "roof", "rookie", "room", "rose", "rotate", "rough", "round",
        "route", "royal", "rubber", "rude", "rug", "rule", "run", "runway", "rural", "sad",
        "saddle", "sadness", "safe", "sail", "salad", "salmon", "salon", "salt", "salute", "same",
        "sample", "sand", "satisfy", "satoshi", "sauce", "sausage", "save", "say", "scale", "scan",
        "scare", "scatter", "scene", "scheme", "school", "science", "scissors", "scorpion", "scout", "scrap",
        "screen", "script", "scrub", "sea", "search", "season", "seat", "second", "secret", "section",
        "security", "seed", "seek", "segment", "select", "sell", "seminar", "senior", "sense", "sentence",
        "series", "service", "session", "settle", "setup", "seven", "shadow", "shaft", "shallow", "share",
        "shed", "shell", "sheriff", "shield", "shift", "shine", "ship", "shiver", "shock", "shoe",
        "shoot", "shop", "short", "shoulder", "shove", "shrimp", "shrug", "shuffle", "shy", "sibling",
        "sick", "side", "siege", "sight", "sign", "silent", "silk", "silly", "silver", "similar",
        "simple", "since", "sing", "siren", "sister", "situate", "six", "size", "skate", "sketch",
        "ski", "skill", "skin", "skirt", "skull", "slab", "slam", "sleep", "slender", "slice",
        "slide", "slight", "slim", "slogan", "slot", "slow", "slush", "small", "smart", "smile",
        "smoke", "smooth", "snack", "snake", "snap", "sniff", "snow", "soap", "soccer", "social",
        "sock", "soda", "soft", "solar", "soldier", "solid", "solution", "solve", "someone", "song",
        "soon", "sorry", "sort", "soul", "sound", "soup", "source", "south", "space", "spare",
        "spatial", "spawn", "speak", "special", "speed", "spell", "spend", "sphere", "spice", "spider",
        "spike", "spin", "spirit", "split", "spoil", "sponsor", "spoon", "sport", "spot", "spray",
        "spread", "spring", "spy", "square", "squeeze", "squirrel", "stable", "stadium", "staff", "stage",
        "stairs", "stamp", "stand", "start", "state", "stay", "steak", "steel", "stem", "step",
        "stereo", "stick", "still", "sting", "stock", "stomach", "stone", "stool", "story", "stove",
        "strategy", "street", "strike", "strong", "struggle", "student", "stuff", "stumble", "style", "subject",
        "submit", "subway", "success", "such", "sudden", "suffer", "sugar", "suggest", "suit", "summer",
        "sun", "sunny", "sunset", "super", "supply", "supreme", "sure", "surface", "surge", "surprise",
        "surround", "survey", "suspect", "sustain", "swallow", "swamp", "swap", "swarm", "swear", "sweet",
        "swift", "swim", "swing", "switch", "sword", "symbol", "symptom", "syrup", "system", "table",
        "tackle", "tag", "tail", "talent", "talk", "tank", "tape", "target", "task", "taste",
        "tattoo", "taxi", "teach", "team", "tell", "ten", "tenant", "tennis", "tent", "term",
        "test", "text", "thank", "that", "theme", "then", "theory", "there", "they", "thing",
        "this", "thought", "three", "thrive", "throw", "thumb", "thunder", "ticket", "tide", "tiger",
        "tilt", "timber", "time", "tiny", "tip", "tired", "tissue", "title", "toast", "tobacco",
        "today", "toddler", "toe", "together", "toilet", "token", "tomato", "tomorrow", "tone", "tongue",
        "tonight", "tool", "tooth", "top", "topic", "topple", "torch", "tornado", "tortoise", "toss",
        "total", "tourist", "toward", "tower", "town", "toy", "track", "trade", "traffic", "tragic",
        "train", "transfer", "trap", "trash", "travel", "tray", "treat", "tree", "trend", "trial",
        "tribe", "trick", "trigger", "trim", "trip", "trophy", "trouble", "truck", "true", "truly",
        "trumpet", "trust", "truth", "try", "tube", "tuition", "tumble", "tuna", "tunnel", "turkey",
        "turn", "turtle", "twelve", "twenty", "twice", "twin", "twist", "two", "type", "typical",
        "ugly", "umbrella", "unable", "unaware", "uncle", "uncover", "under", "undo", "unfair", "unfold",
        "unhappy", "uniform", "unique", "unit", "universe", "unknown", "unlock", "until", "unusual", "unveil",
        "update", "upgrade", "uphold", "upon", "upper", "upset", "urban", "urge", "usage", "use",
        "used", "useful", "useless", "usual", "utility", "vacant", "vacuum", "vague", "valid", "valley",
        "valve", "van", "vanish", "vapor", "various", "vast", "vault", "vehicle", "velvet", "vendor",
        "venture", "venue", "verb", "verify", "version", "very", "vessel", "veteran", "viable",
        "vibrant", "vicious", "victory", "video", "view", "village", "vintage", "violin", "virtual",
        "virus", "visa", "visit", "visual", "vital", "vivid", "vocal", "voice", "void", "volcano",
        "volume", "vote", "voyage", "wage", "wagon", "wait", "walk", "wall", "walnut", "want",
        "warfare", "warm", "warrior", "wash", "wasp", "waste", "water", "wave", "way", "wealth",
        "weapon", "wear", "weasel", "weather", "web", "wedding", "weekend", "weird", "welcome", "west",
        "wet", "whale", "what", "wheat", "wheel", "when", "where", "whip", "whisper", "wide",
        "width", "wife", "wild", "will", "win", "window", "wine", "wing", "wink", "winner",
        "winter", "wire", "wisdom", "wise", "wish", "witness", "wolf", "woman", "wonder", "wood",
        "wool", "word", "work", "world", "worry", "worth", "wrap", "wreck", "wrestle", "wrist",
        "write", "wrong", "yard", "year", "yellow", "you", "young", "youth", "zebra", "zero",
        "zone", "zoo",
    ];

    let mut rng = rand::thread_rng();
    let mut phrase = Vec::with_capacity(word_count);
    for _ in 0..word_count {
        let idx = rng.next_u32() as usize % words.len();
        phrase.push(words[idx].to_string());
    }
    phrase.join(" ")
}

/// 安全审计修复（发现 17，MEDIUM）：原实现使用 `Sha256` 单次哈希（无 salt、无迭代），
/// 易受彩虹表/暴力破解攻击。恢复短语虽由 BIP39 词表随机生成，但用户可能复用/弱化，
/// 且 SHA-256 速度极快（GPU 每秒数十亿次），不适合口令存储。
///
/// 现改用 Argon2id（与 `hash_password` 一致），自动生成随机 salt 并以 PHC 字符串格式
/// 返回（`$argon2id$v=19$m=...`），便于 `verify_recovery_phrase` 调用 `verify_password` 校验。
///
/// **签名不变**：调用方（`auth_service.rs` / `crypto_service.rs`）无需修改。
/// **存储兼容**：`recovery_phrase_hash` 列为 `TEXT`，PHC 字符串（~96 字符）可完整存入。
/// **旧数据迁移**：旧 SHA-256 hex（64 字符）在 `verify_recovery_phrase` 中会因 PHC 解析
/// 失败而返回 `false`，需用户重新设置恢复短语；不自动迁移以避免降级攻击。
pub fn hash_recovery_phrase(phrase: &str) -> Result<String, AppError> {
    // 随机 16 字节 salt（与 generate_salt 同长度，但独立生成避免与 password salt 复用）
    let mut salt_bytes = [0u8; SALT_LENGTH];
    rand::thread_rng().fill_bytes(&mut salt_bytes);
    let salt_string = SaltString::encode_b64(&salt_bytes)
        .map_err(|e| AppError::Crypto(format!("Salt 编码失败: {}", e)))?;

    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(phrase.as_bytes(), &salt_string)
        .map_err(|e| AppError::Crypto(format!("恢复短语哈希失败: {}", e)))?;

    Ok(password_hash.to_string())
}

pub fn verify_recovery_phrase(phrase: &str, stored_hash: &str) -> Result<bool, AppError> {
    // Argon2 PHC 字符串校验：若 stored_hash 不是 PHC 格式（如旧 SHA-256 hex），
    // `PasswordHash::new` 会失败，此处返回 false（拒绝旧哈希登录）而非报错，
    // 避免攻击者构造非法哈希触发 500 错误。
    let parsed_hash = match PasswordHash::new(stored_hash) {
        Ok(h) => h,
        Err(_) => return Ok(false),
    };

    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(phrase.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn derive_kek_from_stored(password_hash: &str) -> Result<[u8; KEK_LENGTH], AppError> {
    let parsed = PasswordHash::new(password_hash)
        .map_err(|e| AppError::Crypto(format!("密码哈希解析失败: {}", e)))?;

    let hash_bytes = parsed.hash.ok_or_else(|| AppError::Crypto("哈希结果为空".into()))?;

    let mut hasher = Sha256::new();
    hasher.update(hash_bytes.as_bytes());
    let result = hasher.finalize();

    let mut kek = [0u8; KEK_LENGTH];
    kek.copy_from_slice(&result[..KEK_LENGTH]);
    Ok(kek)
}

#[cfg(test)]
mod tests {
    //! key_derivation 模块单元测试（v1.52 测试体系 Phase 2 - 2.5.1）
    //!
    //! 覆盖：salt 生成、KEK 派生、密码哈希/校验、恢复短语生成/哈希/校验、从存储哈希派生 KEK
    //! 参见：03_测试体系_单元与集成测试.md §2.2.1（crypto 优先级 P0）

    use super::*;

    // ===== Salt 生成（3 用例）=====

    #[test]
    fn test_generate_salt_length_is_16_bytes() {
        let salt = generate_salt();
        assert_eq!(salt.len(), SALT_LENGTH, "salt 长度应为 {}", SALT_LENGTH);
    }

    #[test]
    fn test_generate_salt_is_random() {
        let s1 = generate_salt();
        let s2 = generate_salt();
        assert_ne!(s1, s2, "两次生成的 salt 不应相同");
    }

    #[test]
    fn test_generate_salt_not_all_zero() {
        let salt = generate_salt();
        assert!(salt.iter().any(|&b| b != 0), "salt 不应全为 0");
    }

    // ===== KEK 派生（4 用例）=====

    #[test]
    fn test_derive_kek_returns_32_bytes() {
        let salt = generate_salt();
        let kek = derive_kek("password", &salt).unwrap();
        assert_eq!(kek.len(), KEK_LENGTH, "KEK 长度应为 {}", KEK_LENGTH);
    }

    #[test]
    fn test_derive_kek_deterministic_for_same_input() {
        let salt = b"salt_value_16byt".to_vec();
        let k1 = derive_kek("password", &salt).unwrap();
        let k2 = derive_kek("password", &salt).unwrap();
        assert_eq!(k1, k2, "相同 password + salt → 相同 KEK");
    }

    #[test]
    fn test_derive_kek_differs_for_different_password() {
        let salt = b"salt_value_16byt".to_vec();
        let k1 = derive_kek("pass1", &salt).unwrap();
        let k2 = derive_kek("pass2", &salt).unwrap();
        assert_ne!(k1, k2, "不同 password → 不同 KEK");
    }

    #[test]
    fn test_derive_kek_differs_for_different_salt() {
        let s1 = b"salt_value_16byt".to_vec();
        let s2 = b"salt_value_16byu".to_vec();
        let k1 = derive_kek("password", &s1).unwrap();
        let k2 = derive_kek("password", &s2).unwrap();
        assert_ne!(k1, k2, "不同 salt → 不同 KEK");
    }

    // ===== 密码哈希/校验（4 用例）=====

    #[test]
    fn test_hash_password_returns_argon2_string() {
        let salt = generate_salt();
        let hash = hash_password("StrongPass!1", &salt).unwrap();
        assert!(!hash.is_empty());
        assert!(hash.starts_with("$argon2"), "应以 $argon2 开头");
    }

    #[test]
    fn test_hash_password_differs_for_different_password() {
        let salt = generate_salt();
        let h1 = hash_password("pass1", &salt).unwrap();
        let h2 = hash_password("pass2", &salt).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_verify_password_correct() {
        let salt = generate_salt();
        let hash = hash_password("CorrectPass!1", &salt).unwrap();
        assert!(verify_password("CorrectPass!1", &hash).unwrap());
    }

    #[test]
    fn test_verify_password_incorrect() {
        let salt = generate_salt();
        let hash = hash_password("CorrectPass!1", &salt).unwrap();
        assert!(!verify_password("WrongPass!1", &hash).unwrap());
    }

    // ===== 恢复短语（5 用例）=====

    #[test]
    fn test_generate_recovery_phrase_word_count() {
        let p12 = generate_recovery_phrase(12);
        assert_eq!(p12.split_whitespace().count(), 12);
        let p24 = generate_recovery_phrase(24);
        assert_eq!(p24.split_whitespace().count(), 24);
    }

    #[test]
    fn test_generate_recovery_phrase_zero_words() {
        assert_eq!(generate_recovery_phrase(0), "");
    }

    #[test]
    fn test_generate_recovery_phrase_is_random() {
        let p1 = generate_recovery_phrase(12);
        let p2 = generate_recovery_phrase(12);
        assert_ne!(p1, p2, "两次生成的短语应不同");
    }

    #[test]
    fn test_hash_recovery_phrase_is_argon2_phc_and_non_deterministic() {
        // 安全审计修复（发现 17）后：Argon2id PHC 字符串格式，每次生成带随机 salt
        let phrase = "abandon ability able about above absent absorb abstract absurd abuse access accident";
        let h1 = hash_recovery_phrase(phrase).unwrap();
        let h2 = hash_recovery_phrase(phrase).unwrap();
        // Argon2 每次生成随机 salt → 相同短语哈希结果不同（防彩虹表）
        assert_ne!(h1, h2, "Argon2id 带随机 salt，相同短语 → 不同哈希");
        // PHC 字符串格式校验
        assert!(h1.starts_with("$argon2"), "应以 $argon2 开头（PHC 格式）");
        assert!(h2.starts_with("$argon2"), "应以 $argon2 开头（PHC 格式）");
        // PHC 字符串典型长度约 96 字符（含 v/m/t/p/salt/hash 参数）
        assert!(h1.len() > 80, "Argon2 PHC 字符串应 > 80 字符，实际: {}", h1.len());
    }

    #[test]
    fn test_verify_recovery_phrase_correct_and_incorrect() {
        let phrase = "abandon ability able about above absent absorb abstract absurd abuse access accident";
        let hash = hash_recovery_phrase(phrase).unwrap();
        assert!(verify_recovery_phrase(phrase, &hash).unwrap());
        assert!(!verify_recovery_phrase("wrong phrase", &hash).unwrap());
    }

    #[test]
    fn test_verify_recovery_phrase_rejects_legacy_sha256_hash() {
        // 安全审计修复（发现 17）：旧 SHA-256 hex 哈希不应被验证通过（防降级攻击）
        // 旧实现：hash_recovery_phrase 返回 64 字符 hex（无 salt）
        // 新实现：返回 Argon2id PHC 字符串（带 salt）
        // 旧的 SHA-256 哈希不是合法 PHC 格式 → verify 应返回 false 而非报错
        let phrase = "abandon ability able about above absent absorb abstract absurd abuse access accident";
        // 模拟旧 SHA-256 哈希（64 字符 hex）
        let legacy_sha256_hex = "a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef12345678";
        assert!(
            !verify_recovery_phrase(phrase, legacy_sha256_hex).unwrap(),
            "旧 SHA-256 hex 哈希应被拒绝（PHC 解析失败 → false）"
        );
    }

    // ===== 从存储哈希派生 KEK（2 用例）=====

    #[test]
    fn test_derive_kek_from_stored_returns_32_bytes() {
        let salt = generate_salt();
        let password_hash = hash_password("mypassword", &salt).unwrap();
        let kek = derive_kek_from_stored(&password_hash).unwrap();
        assert_eq!(kek.len(), KEK_LENGTH);
    }

    #[test]
    fn test_derive_kek_from_stored_deterministic() {
        let salt = generate_salt();
        let password_hash = hash_password("mypassword", &salt).unwrap();
        let k1 = derive_kek_from_stored(&password_hash).unwrap();
        let k2 = derive_kek_from_stored(&password_hash).unwrap();
        assert_eq!(k1, k2, "相同存储哈希 → 相同 KEK");
    }
}