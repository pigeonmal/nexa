//! Shared semantic icon names and their AOT platform mappings.
//!
//! The catalog is target-neutral: app code names a concept once, and each
//! backend asks the same definition for its native SF Symbol or Compose vector.

/// A system icon selection lowered from `Icon(system:)`, `Icon(sfsymbol:)`, or
/// `Icon(materialsymbol:)`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemIcon {
    /// A name in [`SHARED_ICONS`], mapped to both native icon systems.
    Shared(String),
    /// An explicit iOS SF Symbol name.
    SfSymbol(String),
    /// An explicit Android Compose Material icon name.
    MaterialSymbol(String),
}

/// One portable icon name and its native spellings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedIconDefinition {
    pub name: &'static str,
    pub sf_symbol: &'static str,
    /// Compose icon namespace, such as `Filled` or `AutoMirrored.Filled`.
    pub material_namespace: &'static str,
    pub material_name: &'static str,
    /// Native spellings that can be translated through this definition when an
    /// explicit platform symbol is lowered for the other target.
    pub aliases: &'static [&'static str],
}

/// A Material vector resource usable by Android Glance widget rendering.
/// Glance serializes `RemoteViews` and cannot render Compose `ImageVector`
/// values directly, so Nexa emits this path as a package-owned drawable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AndroidWidgetDrawable {
    pub resource_name: &'static str,
    pub path_data: &'static str,
}

/// Portable icon names supported by `Icon(system:)` and tab `icon:` values.
///
/// Keep this table as the single source of truth for both AOT backends. Names
/// are lowercase semantic identifiers. `system:` accepts only `name`; aliases
/// support native-name translation when an explicit platform symbol is used.
pub const SHARED_ICONS: &[SharedIconDefinition] = &[
    icon(
        "home",
        "house.fill",
        "Filled",
        "Home",
        &["house", "house.fill"],
    ),
    icon(
        "search",
        "magnifyingglass",
        "Filled",
        "Search",
        &["magnifyingglass"],
    ),
    icon(
        "inbox",
        "tray.fill",
        "Filled",
        "Inbox",
        &["tray", "tray.fill"],
    ),
    icon("person", "person", "Filled", "Person", &["person.fill"]),
    icon("people", "person.2", "Filled", "Group", &["person.2.fill"]),
    icon(
        "favorite",
        "heart",
        "Outlined",
        "FavoriteBorder",
        &["heart"],
    ),
    icon(
        "favorite_filled",
        "heart.fill",
        "Filled",
        "Favorite",
        &["heart.fill"],
    ),
    icon(
        "comment",
        "bubble.right",
        "Outlined",
        "ChatBubbleOutline",
        &["bubble.right"],
    ),
    icon(
        "comment_left",
        "bubble.left",
        "Outlined",
        "ChatBubbleOutline",
        &["bubble.left"],
    ),
    icon(
        "comment_filled",
        "bubble.right.fill",
        "Filled",
        "ChatBubble",
        &["bubble.left.fill", "bubble.right.fill"],
    ),
    icon(
        "bookmark",
        "bookmark",
        "Outlined",
        "BookmarkBorder",
        &["bookmark"],
    ),
    icon(
        "bookmark_filled",
        "bookmark.fill",
        "Filled",
        "Bookmark",
        &["bookmark.fill"],
    ),
    icon(
        "share",
        "square.and.arrow.up",
        "Filled",
        "Share",
        &["arrowshape.turn.up.right"],
    ),
    icon(
        "music",
        "music.note",
        "Filled",
        "MusicNote",
        &["music.note"],
    ),
    icon(
        "back",
        "chevron.left",
        "AutoMirrored.Filled",
        "ArrowBack",
        &["chevron.left", "arrow.left"],
    ),
    icon(
        "forward",
        "chevron.right",
        "AutoMirrored.Filled",
        "ArrowForward",
        &["chevron.right", "arrow.right"],
    ),
    icon("tv", "tv", "Filled", "Tv", &["tv"]),
    icon(
        "layers",
        "square.3.layers.3d",
        "Filled",
        "Layers",
        &["rectangle.on.rectangle"],
    ),
    icon("add", "plus", "Filled", "Add", &["plus"]),
    icon("close", "xmark", "Filled", "Close", &["xmark"]),
    icon("check", "checkmark", "Filled", "Check", &["checkmark"]),
    icon(
        "check_circle",
        "checkmark.circle",
        "Outlined",
        "CheckCircleOutline",
        &["checkmark.circle"],
    ),
    icon(
        "check_circle_filled",
        "checkmark.circle.fill",
        "Filled",
        "CheckCircle",
        &["checkmark.circle.fill"],
    ),
    icon("send", "paperplane", "Filled", "Send", &["paperplane"]),
    icon(
        "submit",
        "arrow.up.circle.fill",
        "AutoMirrored.Filled",
        "Send",
        &["arrow.up.circle.fill"],
    ),
    icon(
        "volume_up",
        "speaker.wave.2.fill",
        "Filled",
        "VolumeUp",
        &["speaker.wave.2.fill"],
    ),
    icon(
        "volume_off",
        "speaker.slash.fill",
        "AutoMirrored.Filled",
        "VolumeOff",
        &["speaker.slash.fill"],
    ),
    icon(
        "calendar",
        "calendar",
        "Outlined",
        "CalendarMonth",
        &["calendar"],
    ),
    icon(
        "calendar_circle",
        "calendar.circle",
        "Filled",
        "CalendarMonth",
        &["calendar.circle"],
    ),
    icon(
        "event_add",
        "calendar.badge.plus",
        "Filled",
        "EventAvailable",
        &["calendar.badge.plus"],
    ),
    icon("sun", "sun.max.fill", "Filled", "WbSunny", &["sun.max"]),
    icon(
        "event_done",
        "calendar.badge.checkmark",
        "Filled",
        "EventAvailable",
        &["calendar.badge.checkmark"],
    ),
    icon(
        "calendar_time",
        "calendar.badge.clock",
        "Filled",
        "Event",
        &["calendar.badge.clock"],
    ),
    icon(
        "sun_filled",
        "sun.max.fill",
        "Filled",
        "WbSunny",
        &["sun.max.fill"],
    ),
    icon("sunrise", "sunrise", "Filled", "WbTwilight", &["sunrise"]),
    icon(
        "settings",
        "gearshape.fill",
        "Filled",
        "Settings",
        &["gearshape.fill", "gearshape"],
    ),
    icon(
        "sort",
        "arrow.up.arrow.down",
        "Filled",
        "Sort",
        &["arrow.up.arrow.down"],
    ),
    icon(
        "notifications",
        "bell.fill",
        "Filled",
        "Notifications",
        &["bell", "bell.fill"],
    ),
    icon(
        "notifications_active",
        "bell.badge.fill",
        "Filled",
        "NotificationsActive",
        &["bell.badge", "bell.badge.fill"],
    ),
    icon(
        "checklist",
        "checklist",
        "Filled",
        "AssignmentTurnedIn",
        &["checklist"],
    ),
    icon(
        "circle",
        "circle",
        "Filled",
        "RadioButtonUnchecked",
        &["circle"],
    ),
    icon(
        "circle_filled",
        "circle.fill",
        "Filled",
        "Circle",
        &["circle.fill"],
    ),
    icon("star", "star.fill", "Filled", "Star", &["star"]),
    icon("star_filled", "star.fill", "Filled", "Star", &["star.fill"]),
    icon("flag", "flag", "Filled", "Flag", &[]),
    icon("flag_filled", "flag.fill", "Filled", "Flag", &["flag.fill"]),
    icon(
        "delete",
        "trash",
        "Filled",
        "Delete",
        &["trash.fill", "trash"],
    ),
    icon("edit", "pencil", "Filled", "Edit", &["pencil"]),
    icon(
        "more",
        "ellipsis",
        "Filled",
        "MoreHoriz",
        &["ellipsis.circle", "ellipsis"],
    ),
    icon(
        "attach",
        "paperclip",
        "Filled",
        "AttachFile",
        &["paperclip"],
    ),
    icon(
        "image",
        "photo",
        "Filled",
        "Image",
        &["photo.fill", "photo"],
    ),
    icon(
        "camera",
        "camera",
        "Filled",
        "CameraAlt",
        &["camera.fill", "camera"],
    ),
    icon(
        "clock",
        "clock",
        "Filled",
        "Schedule",
        &["clock.fill", "clock"],
    ),
    icon("map", "map", "Filled", "Map", &["map.fill", "map"]),
    icon(
        "location",
        "location",
        "Filled",
        "LocationOn",
        &["location.fill", "location"],
    ),
    icon(
        "email",
        "envelope.fill",
        "Filled",
        "Email",
        &["envelope.fill", "envelope"],
    ),
    icon("phone", "phone", "Filled", "Call", &["phone.fill", "phone"]),
    icon("link", "link", "Filled", "Link", &["link"]),
    icon(
        "visibility",
        "eye",
        "Filled",
        "Visibility",
        &["eye.fill", "eye"],
    ),
    icon(
        "visibility_off",
        "eye.slash",
        "Filled",
        "VisibilityOff",
        &["eye.slash"],
    ),
    icon("lock", "lock", "Filled", "Lock", &["lock.fill", "lock"]),
    icon(
        "security",
        "checkmark.shield.fill",
        "Filled",
        "GppGood",
        &["lock.shield", "checkmark.shield.fill"],
    ),
    icon(
        "document",
        "doc.text.fill",
        "Filled",
        "Description",
        &["doc.text", "doc.text.fill"],
    ),
    icon("language", "globe", "Filled", "Language", &["safari"]),
    icon(
        "palette",
        "paintpalette.fill",
        "Filled",
        "Palette",
        &["paintpalette", "paintpalette.fill"],
    ),
    icon(
        "code",
        "chevron.left.forwardslash.chevron.right",
        "Filled",
        "Code",
        &["curlybraces"],
    ),
    icon(
        "party_popper",
        "party.popper",
        "Filled",
        "Celebration",
        &["party.popper"],
    ),
    icon(
        "speedometer",
        "speedometer",
        "Filled",
        "Speed",
        &["speedometer"],
    ),
    icon(
        "waving_hand",
        "hand.wave.fill",
        "Filled",
        "WavingHand",
        &["hand.wave", "hand.wave.fill"],
    ),
    // Common navigation, editing, media, and status icons. Keep names
    // semantic and stable so app source never needs either native spelling.
    icon(
        "arrow_back",
        "arrow.left",
        "AutoMirrored.Filled",
        "ArrowBack",
        &["arrow_back"],
    ),
    icon(
        "arrow_forward",
        "arrow.right",
        "AutoMirrored.Filled",
        "ArrowForward",
        &["arrow_forward"],
    ),
    icon(
        "arrow_up",
        "arrow.up",
        "Filled",
        "ArrowUpward",
        &["arrow_upward"],
    ),
    icon(
        "arrow_down",
        "arrow.down",
        "Filled",
        "ArrowDownward",
        &["arrow_downward"],
    ),
    icon(
        "menu",
        "line.3.horizontal",
        "Filled",
        "Menu",
        &["line.3.horizontal"],
    ),
    icon(
        "dashboard",
        "square.grid.2x2",
        "Filled",
        "Dashboard",
        &["square.grid.2x2"],
    ),
    icon(
        "grid",
        "square.grid.3x3",
        "Filled",
        "GridView",
        &["square.grid.3x3"],
    ),
    icon(
        "list",
        "list.bullet",
        "Filled",
        "ViewList",
        &["list.bullet"],
    ),
    icon(
        "refresh",
        "arrow.clockwise",
        "Filled",
        "Refresh",
        &["arrow.clockwise"],
    ),
    icon(
        "undo",
        "arrow.uturn.backward",
        "Filled",
        "Undo",
        &["arrow.uturn.backward"],
    ),
    icon(
        "redo",
        "arrow.uturn.forward",
        "Filled",
        "Redo",
        &["arrow.uturn.forward"],
    ),
    icon(
        "filter",
        "line.3.horizontal.decrease",
        "Filled",
        "FilterList",
        &["line.3.horizontal.decrease"],
    ),
    icon(
        "tune",
        "slider.horizontal.3",
        "Filled",
        "Tune",
        &["slider.horizontal.3"],
    ),
    icon(
        "download",
        "arrow.down.to.line",
        "Filled",
        "FileDownload",
        &["arrow.down.to.line"],
    ),
    icon(
        "upload",
        "arrow.up.to.line",
        "Filled",
        "FileUpload",
        &["arrow.up.to.line"],
    ),
    icon(
        "open",
        "arrow.up.right.square",
        "Filled",
        "OpenInNew",
        &["arrow.up.right.square"],
    ),
    icon(
        "copy",
        "doc.on.doc",
        "Filled",
        "ContentCopy",
        &["doc.on.doc"],
    ),
    icon(
        "paste",
        "doc.on.clipboard",
        "Filled",
        "ContentPaste",
        &["doc.on.clipboard"],
    ),
    icon("cut", "scissors", "Filled", "ContentCut", &["scissors"]),
    icon(
        "save",
        "square.and.arrow.down",
        "Filled",
        "Save",
        &["square.and.arrow.down"],
    ),
    icon("print", "printer", "Filled", "Print", &["printer"]),
    icon(
        "help",
        "questionmark.circle",
        "Outlined",
        "HelpOutline",
        &["questionmark.circle"],
    ),
    icon("info", "info.circle", "Filled", "Info", &["info.circle"]),
    icon(
        "warning",
        "exclamationmark.triangle",
        "Filled",
        "Warning",
        &["exclamationmark.triangle"],
    ),
    icon(
        "error",
        "xmark.octagon",
        "Filled",
        "Error",
        &["xmark.octagon"],
    ),
    icon(
        "verified",
        "checkmark.seal",
        "Filled",
        "Verified",
        &["checkmark.seal"],
    ),
    icon(
        "sync",
        "arrow.triangle.2.circlepath",
        "Filled",
        "Sync",
        &["arrow.triangle.2.circlepath"],
    ),
    icon("cloud", "cloud", "Filled", "Cloud", &["cloud"]),
    icon(
        "cloud_upload",
        "icloud.and.arrow.up",
        "Filled",
        "CloudUpload",
        &["icloud.and.arrow.up"],
    ),
    icon(
        "cloud_download",
        "icloud.and.arrow.down",
        "Filled",
        "CloudDownload",
        &["icloud.and.arrow.down"],
    ),
    icon("wifi", "wifi", "Filled", "Wifi", &["wifi"]),
    icon(
        "bluetooth",
        "bluetooth",
        "Filled",
        "Bluetooth",
        &["bluetooth"],
    ),
    icon(
        "battery",
        "battery.100",
        "Filled",
        "BatteryFull",
        &["battery.100"],
    ),
    icon("airplane", "airplane", "Filled", "Flight", &["airplane"]),
    icon("flash", "bolt", "Filled", "Bolt", &["bolt"]),
    icon(
        "flash_filled",
        "bolt.fill",
        "Filled",
        "FlashOn",
        &["bolt.fill"],
    ),
    icon("dark_mode", "moon", "Filled", "DarkMode", &["moon"]),
    icon("light_mode", "sun.max", "Filled", "LightMode", &[]),
    icon("work", "briefcase", "Filled", "Work", &["briefcase"]),
    icon(
        "school",
        "graduationcap",
        "Filled",
        "School",
        &["graduationcap"],
    ),
    icon(
        "home_work",
        "house.and.flag",
        "Filled",
        "HomeWork",
        &["house.and.flag"],
    ),
    icon("shopping_cart", "cart", "Filled", "ShoppingCart", &["cart"]),
    icon("shopping_bag", "bag", "Filled", "ShoppingBag", &["bag"]),
    icon(
        "payment",
        "creditcard",
        "Filled",
        "CreditCard",
        &["creditcard"],
    ),
    icon("receipt", "receipt", "Filled", "Receipt", &["receipt"]),
    icon(
        "wallet",
        "wallet.pass",
        "Filled",
        "AccountBalanceWallet",
        &["wallet.pass"],
    ),
    icon("gift", "gift", "Filled", "CardGiftcard", &["gift"]),
    icon(
        "restaurant",
        "fork.knife",
        "Filled",
        "Restaurant",
        &["fork.knife"],
    ),
    icon(
        "coffee",
        "cup.and.saucer",
        "Filled",
        "Coffee",
        &["cup.and.saucer"],
    ),
    icon(
        "fitness",
        "figure.run",
        "Filled",
        "FitnessCenter",
        &["figure.run"],
    ),
    icon(
        "health",
        "heart.text.square",
        "Filled",
        "HealthAndSafety",
        &["heart.text.square"],
    ),
    icon("pets", "pawprint", "Filled", "Pets", &["pawprint"]),
    icon(
        "travel",
        "suitcase.rolling",
        "Filled",
        "Luggage",
        &["suitcase.rolling"],
    ),
    icon(
        "directions",
        "location.north.line",
        "Filled",
        "Navigation",
        &["location.north.line"],
    ),
    icon(
        "place",
        "mappin.and.ellipse",
        "Filled",
        "Place",
        &["mappin.and.ellipse"],
    ),
    icon("menu_book", "book", "Filled", "MenuBook", &["book"]),
    icon(
        "bookmarks",
        "books.vertical",
        "Filled",
        "CollectionsBookmark",
        &["books.vertical"],
    ),
    icon("folder", "folder", "Filled", "Folder", &["folder"]),
    icon(
        "folder_open",
        "folder",
        "Filled",
        "FolderOpen",
        &["folder.open"],
    ),
    icon("file", "doc", "Filled", "InsertDriveFile", &["doc"]),
    icon(
        "pdf",
        "doc.richtext",
        "Filled",
        "PictureAsPdf",
        &["doc.richtext"],
    ),
    icon(
        "task",
        "checkmark.square",
        "Filled",
        "TaskAlt",
        &["checkmark.square"],
    ),
    icon("event", "calendar", "Filled", "Event", &[]),
    icon("alarm", "alarm", "Filled", "Alarm", &["alarm"]),
    icon("timer", "timer", "Filled", "Timer", &["timer"]),
    icon("play", "play.fill", "Filled", "PlayArrow", &["play.fill"]),
    icon("pause", "pause.fill", "Filled", "Pause", &["pause.fill"]),
    icon("stop", "stop.fill", "Filled", "Stop", &["stop.fill"]),
    icon(
        "skip_next",
        "forward.end.fill",
        "Filled",
        "SkipNext",
        &["forward.end.fill"],
    ),
    icon(
        "skip_previous",
        "backward.end.fill",
        "Filled",
        "SkipPrevious",
        &["backward.end.fill"],
    ),
    icon(
        "volume_down",
        "speaker.wave.1.fill",
        "Filled",
        "VolumeDown",
        &["speaker.wave.1.fill"],
    ),
    icon(
        "mute",
        "speaker.slash",
        "Filled",
        "VolumeMute",
        &["speaker.slash"],
    ),
    icon(
        "headphones",
        "headphones",
        "Filled",
        "Headphones",
        &["headphones"],
    ),
    icon("microphone", "mic", "Filled", "Mic", &["mic"]),
    icon(
        "microphone_off",
        "mic.slash",
        "Filled",
        "MicOff",
        &["mic.slash"],
    ),
    icon("movie", "film", "Filled", "Movie", &["film"]),
    icon(
        "music_library",
        "music.note.list",
        "Filled",
        "LibraryMusic",
        &["music.note.list"],
    ),
    icon(
        "chat",
        "bubble.left.and.bubble.right",
        "Filled",
        "Forum",
        &["bubble.left.and.bubble.right"],
    ),
    icon(
        "chat_filled",
        "bubble.left.and.bubble.right.fill",
        "Filled",
        "Forum",
        &["bubble.left.and.bubble.right.fill"],
    ),
    icon(
        "group_add",
        "person.2.badge.plus",
        "Filled",
        "GroupAdd",
        &["person.2.badge.plus"],
    ),
    icon(
        "account_circle",
        "person.crop.circle",
        "Filled",
        "AccountCircle",
        &["person.crop.circle"],
    ),
    icon(
        "logout",
        "rectangle.portrait.and.arrow.right",
        "Filled",
        "Logout",
        &["rectangle.portrait.and.arrow.right"],
    ),
    icon(
        "login",
        "rectangle.portrait.and.arrow.left",
        "Filled",
        "Login",
        &["rectangle.portrait.and.arrow.left"],
    ),
    icon(
        "lock_open",
        "lock.open",
        "Filled",
        "LockOpen",
        &["lock.open"],
    ),
    icon("key", "key", "Filled", "Key", &["key"]),
    icon("shield", "shield", "Filled", "Shield", &["shield"]),
    icon(
        "wifi_off",
        "wifi.slash",
        "Filled",
        "WifiOff",
        &["wifi.slash"],
    ),
    // Additional portable names for common navigation, media, device, and domain concepts.
    icon("add_circle", "plus.circle", "Filled", "AddCircle", &[]),
    icon(
        "remove_circle",
        "minus.circle",
        "Filled",
        "RemoveCircle",
        &[],
    ),
    icon("cancel", "xmark.circle", "Filled", "Cancel", &[]),
    icon(
        "expand",
        "arrow.up.left.and.arrow.down.right",
        "Filled",
        "OpenInFull",
        &[],
    ),
    icon(
        "collapse",
        "arrow.down.right.and.arrow.up.left",
        "Filled",
        "CloseFullscreen",
        &[],
    ),
    icon("chevron_up", "chevron.up", "Filled", "KeyboardArrowUp", &[]),
    icon(
        "chevron_down",
        "chevron.down",
        "Filled",
        "KeyboardArrowDown",
        &[],
    ),
    icon("chevron_left", "chevron.left", "Filled", "ChevronLeft", &[]),
    icon(
        "chevron_right",
        "chevron.right",
        "Filled",
        "ChevronRight",
        &[],
    ),
    icon(
        "open_in_new",
        "arrow.up.right.square",
        "Filled",
        "OpenInNew",
        &[],
    ),
    icon("directions_car", "car", "Filled", "DirectionsCar", &[]),
    icon(
        "directions_bike",
        "bicycle",
        "Filled",
        "DirectionsBike",
        &[],
    ),
    icon(
        "directions_walk",
        "figure.walk",
        "Filled",
        "DirectionsWalk",
        &[],
    ),
    icon("train", "tram", "Filled", "Train", &[]),
    icon("bus", "directions_bus", "Filled", "DirectionsBus", &[]),
    icon("map_pin", "mappin", "Filled", "Place", &[]),
    icon(
        "navigation",
        "location.north.fill",
        "Filled",
        "Navigation",
        &[],
    ),
    icon("weather_cloud", "cloud", "Filled", "Cloud", &[]),
    icon("weather_rain", "cloud.rain", "Filled", "WbCloudy", &[]),
    icon("weather_snow", "cloud.snow", "Filled", "AcUnit", &[]),
    icon(
        "weather_thunder",
        "cloud.bolt",
        "Filled",
        "Thunderstorm",
        &[],
    ),
    icon("weather_night", "moon.stars", "Filled", "NightsStay", &[]),
    icon(
        "thermostat",
        "thermometer.medium",
        "Filled",
        "DeviceThermostat",
        &[],
    ),
    icon("water_drop", "drop", "Filled", "WaterDrop", &[]),
    icon("fire", "flame", "Filled", "Whatshot", &[]),
    icon("eco", "leaf", "Filled", "Eco", &[]),
    icon("lightbulb", "lightbulb", "Filled", "Lightbulb", &[]),
    icon(
        "flashlight",
        "flashlight.on.fill",
        "Filled",
        "FlashlightOn",
        &[],
    ),
    icon("computer", "desktopcomputer", "Filled", "Computer", &[]),
    icon("laptop", "laptopcomputer", "Filled", "Laptop", &[]),
    icon("tablet", "ipad", "Filled", "TabletMac", &[]),
    icon("smartphone", "iphone", "Filled", "Smartphone", &[]),
    icon("keyboard", "keyboard", "Filled", "Keyboard", &[]),
    icon("mouse", "computermouse", "Filled", "Mouse", &[]),
    icon("gamepad", "gamecontroller", "Filled", "SportsEsports", &[]),
    icon(
        "person_add",
        "person.badge.plus",
        "Filled",
        "PersonAdd",
        &[],
    ),
    icon(
        "person_remove",
        "person.badge.minus",
        "Filled",
        "PersonRemove",
        &[],
    ),
    icon("video", "video", "Filled", "Videocam", &[]),
    icon("video_off", "video.slash", "Filled", "VideocamOff", &[]),
    icon("radio", "radio", "Filled", "Radio", &[]),
    icon(
        "equalizer",
        "slider.horizontal.3",
        "Filled",
        "GraphicEq",
        &[],
    ),
    icon("qr_code", "qrcode", "Filled", "QrCode", &[]),
    icon("barcode", "barcode", "Filled", "Scanner", &[]),
    icon("analytics", "chart.bar", "Filled", "BarChart", &[]),
    icon("chart_pie", "chart.pie", "Filled", "PieChart", &[]),
    icon(
        "trending_up",
        "chart.line.uptrend.xyaxis",
        "Filled",
        "TrendingUp",
        &[],
    ),
    icon(
        "trending_down",
        "chart.line.downtrend.xyaxis",
        "Filled",
        "TrendingDown",
        &[],
    ),
    icon("medical", "cross.case", "Filled", "MedicalServices", &[]),
    icon("medication", "pills", "Filled", "Medication", &[]),
    icon("vaccines", "syringe", "Filled", "Vaccines", &[]),
    icon("science", "atom", "Filled", "Science", &[]),
    icon("calculate", "function", "Filled", "Calculate", &[]),
    icon("translate", "translate", "Filled", "Translate", &[]),
    icon("accessibility", "figure.roll", "Filled", "Accessible", &[]),
    icon(
        "baby",
        "figure.and.child.holdinghands",
        "Filled",
        "ChildCare",
        &[],
    ),
    icon("construction", "hammer", "Filled", "Construction", &[]),
    icon("apartment", "building", "Filled", "Apartment", &[]),
    icon("hotel", "bed.double", "Filled", "Hotel", &[]),
    icon("savings", "banknote", "Filled", "Savings", &[]),
    icon(
        "account_balance",
        "building.columns",
        "Filled",
        "AccountBalance",
        &[],
    ),
    icon("percent", "percent", "Filled", "Percent", &[]),
    icon("note", "note.text", "Filled", "Note", &[]),
    icon("sticky_note", "note", "Filled", "StickyNote2", &[]),
    icon(
        "bookmark_add",
        "bookmark.circle",
        "Filled",
        "BookmarkAdd",
        &[],
    ),
    icon("push_pin", "pin", "Filled", "PushPin", &[]),
    icon(
        "history",
        "clock.arrow.circlepath",
        "Filled",
        "History",
        &[],
    ),
    icon("snooze", "zzz", "Filled", "Snooze", &[]),
    icon(
        "event_busy",
        "calendar.badge.exclamationmark",
        "Filled",
        "EventBusy",
        &[],
    ),
    icon("call_end", "phone.down", "Filled", "CallEnd", &[]),
    icon("message", "bubble.left", "Filled", "Message", &[]),
    icon(
        "notifications_off",
        "bell.slash",
        "Filled",
        "NotificationsOff",
        &[],
    ),
    icon("privacy", "hand.raised", "Filled", "PrivacyTip", &[]),
    icon("zoom_in", "plus.magnifyingglass", "Filled", "ZoomIn", &[]),
    icon(
        "zoom_out",
        "minus.magnifyingglass",
        "Filled",
        "ZoomOut",
        &[],
    ),
];

const fn icon(
    name: &'static str,
    sf_symbol: &'static str,
    material_namespace: &'static str,
    material_name: &'static str,
    aliases: &'static [&'static str],
) -> SharedIconDefinition {
    SharedIconDefinition {
        name,
        sf_symbol,
        material_namespace,
        material_name,
        aliases,
    }
}

/// Compose spelling of a shared icon, such as `AutoMirrored.Filled.ArrowBack`.
///
/// This is the same `namespace.Name` form the Android backend emits, built from
/// the two catalog fields rather than re-spelled per consumer, so the published
/// icon table cannot drift from what `Icons.<namespace>.<name>` resolves to at
/// code generation time.
pub fn material_symbol_path(definition: &SharedIconDefinition) -> String {
    format!(
        "{}.{}",
        definition.material_namespace, definition.material_name
    )
}

/// Render the shared icon mapping table published as
/// `docs/system-icons.md#icons`.
///
/// The table restates [`SHARED_ICONS`] row for row -- four columns, one row per
/// definition, in catalog order -- because a hand-copied list of ~230 platform
/// mappings is a list that will be missing an entry. A test asserts the
/// published region matches this output, so an icon added to the catalog without
/// a documentation update fails the build instead of shipping undocumented.
pub fn render_shared_icon_table() -> String {
    let mut out = String::from(
        "| Shared Semantic Name | iOS SF Symbol | Android Material Icon | Native Aliases |\n\
         |---|---|---|---|\n",
    );
    for definition in SHARED_ICONS {
        let aliases = if definition.aliases.is_empty() {
            "`—`".to_owned()
        } else {
            definition
                .aliases
                .iter()
                .map(|alias| format!("`{alias}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | {aliases} |\n",
            definition.name,
            definition.sf_symbol,
            material_symbol_path(definition),
        ));
    }
    out
}

impl SystemIcon {
    /// Returns the Glance drawable definition for shared icons with a vector
    /// path in Nexa's widget-safe catalog. Unknown and platform-only symbols
    /// return `None`; callers must report an unsupported-icon diagnostic.
    pub fn android_widget_drawable(&self) -> Option<AndroidWidgetDrawable> {
        let shared = match self {
            Self::Shared(name) => SHARED_ICONS.iter().find(|icon| icon.name == name.as_str()),
            Self::SfSymbol(name) => SHARED_ICONS.iter().find(|icon| {
                icon.sf_symbol == name || icon.aliases.iter().any(|alias| *alias == name)
            }),
            Self::MaterialSymbol(name) => SHARED_ICONS.iter().find(|icon| {
                icon.material_name == name
                    || name
                        .rsplit('.')
                        .next()
                        .is_some_and(|suffix| suffix == icon.material_name)
            }),
        }?;

        let (resource_name, path_data) = match shared.name {
            "calendar" => (
                "nexa_widget_calendar",
                "M19 4h-1V2h-2v2H8V2H6v2H5C3.9 4 3 4.9 3 6v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 16H5v-9h14v9zM7 12h5v5H7z",
            ),
            "calendar_circle" => (
                "nexa_widget_calendar_circle",
                "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm5-12h-1V6h-2v2h-4V6H8v2H7v9h10V8zm-2 7H9v-5h6v5z",
            ),
            "calendar_time" => (
                "nexa_widget_calendar_time",
                "M17 12h-5v5h5v-5zm3-9h-1V1h-2v2H7V1H5v2H4c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H4V8h16v11z",
            ),
            "check_circle" | "check_circle_filled" => (
                "nexa_widget_check_circle",
                "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z",
            ),
            "circle" => (
                "nexa_widget_circle",
                "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z",
            ),
            "circle_filled" => (
                "nexa_widget_circle_filled",
                "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2z",
            ),
            _ => return None,
        };
        Some(AndroidWidgetDrawable {
            resource_name,
            path_data,
        })
    }

    pub fn shared(name: impl Into<String>) -> Option<Self> {
        let name = name.into();
        shared_icon(&name)
            .filter(|definition| definition.name == name)
            .map(|_| Self::Shared(name))
    }

    pub fn sf_symbol(name: impl Into<String>) -> Option<Self> {
        let name = name.into();
        (!name.trim().is_empty() && !name.contains(['\n', '\r', '\0']))
            .then_some(Self::SfSymbol(name))
    }

    pub fn material_symbol(name: impl Into<String>) -> Option<Self> {
        let name = name.into();
        let symbol = material_symbol_parts(&name)?;
        (!symbol.1.is_empty()).then_some(Self::MaterialSymbol(name))
    }

    pub fn sf_symbol_name(&self) -> String {
        match self {
            Self::Shared(name) => shared_icon(name).map_or_else(
                || "questionmark".to_owned(),
                |icon| icon.sf_symbol.to_owned(),
            ),
            Self::SfSymbol(name) => name.clone(),
            Self::MaterialSymbol(name) => shared_icon_by_material(name).map_or_else(
                || "questionmark".to_owned(),
                |icon| icon.sf_symbol.to_owned(),
            ),
        }
    }

    pub fn material_import(&self) -> String {
        match self {
            Self::Shared(name) => shared_icon(name).map_or_else(
                || material_import("Filled", "Star"),
                |icon| material_import(icon.material_namespace, icon.material_name),
            ),
            Self::SfSymbol(name) => shared_icon(name).map_or_else(
                || material_import("Filled", "Star"),
                |icon| material_import(icon.material_namespace, icon.material_name),
            ),
            Self::MaterialSymbol(name) => {
                let (namespace, symbol) =
                    material_symbol_parts(name).unwrap_or(("Filled", "Star".to_owned()));
                material_import(namespace, &symbol)
            }
        }
    }

    pub fn material_reference(&self) -> String {
        let (namespace, symbol) = match self {
            Self::Shared(name) => shared_icon(name)
                .map(|icon| (icon.material_namespace, icon.material_name.to_owned()))
                .unwrap_or_else(|| ("Filled", "Star".to_owned())),
            Self::SfSymbol(name) => shared_icon(name)
                .map(|icon| (icon.material_namespace, icon.material_name.to_owned()))
                .unwrap_or_else(|| ("Filled", "Star".to_owned())),
            Self::MaterialSymbol(name) => {
                material_symbol_parts(name).unwrap_or_else(|| ("Filled", "Star".to_owned()))
            }
        };
        format!("Icons.{namespace}.{symbol}")
    }
}

pub fn shared_icon(name: &str) -> Option<&'static SharedIconDefinition> {
    SHARED_ICONS
        .iter()
        .find(|icon| icon.name == name || icon.aliases.contains(&name))
}

fn shared_icon_by_material(name: &str) -> Option<&'static SharedIconDefinition> {
    let (_, symbol) = material_symbol_parts(name)?;
    SHARED_ICONS
        .iter()
        .find(|icon| icon.material_name == symbol)
}

fn material_import(namespace: &str, symbol: &str) -> String {
    // Compose groups are package segments, including `automirrored.filled`.
    let group = namespace.to_lowercase();
    format!("androidx.compose.material.icons.{group}.{symbol}")
}

fn material_symbol_parts(name: &str) -> Option<(&str, String)> {
    if name.trim().is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-' | ':'))
    {
        return None;
    }
    let (namespace, symbol) = if let Some((style, symbol)) = name.split_once(':') {
        let namespace = match style {
            "filled" => "Filled",
            "outlined" => "Outlined",
            "rounded" => "Rounded",
            "sharp" => "Sharp",
            "twoTone" | "two_tone" => "TwoTone",
            "autoMirroredFilled" => "AutoMirrored.Filled",
            "autoMirroredOutlined" => "AutoMirrored.Outlined",
            _ => return None,
        };
        (namespace, symbol)
    } else {
        ("Filled", name)
    };
    let class_name = symbol
        .split(['_', '.', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + chars.as_str()
            })
        })
        .collect::<String>();
    Some((namespace, class_name))
}

#[cfg(test)]
mod tests {
    use super::{SHARED_ICONS, SystemIcon, shared_icon};

    #[test]
    fn shared_catalog_names_are_unique_and_map_to_both_native_systems() {
        for (index, icon) in SHARED_ICONS.iter().enumerate() {
            assert!(!icon.name.is_empty());
            assert!(!icon.sf_symbol.is_empty());
            assert!(!icon.material_name.is_empty());
            assert!(!icon.material_namespace.is_empty());
            assert!(
                !SHARED_ICONS[..index]
                    .iter()
                    .any(|earlier| earlier.name == icon.name)
            );
        }
        let mut accepted_names = std::collections::BTreeMap::new();
        for icon in SHARED_ICONS {
            assert_eq!(
                accepted_names.insert(icon.name, icon.name),
                None,
                "duplicate icon name: {}",
                icon.name
            );
            for alias in icon.aliases {
                let previous = accepted_names.insert(alias, icon.name);
                assert!(
                    previous.is_none() || previous == Some(icon.name),
                    "ambiguous icon alias: {alias}"
                );
            }
        }
    }

    #[test]
    fn shared_names_are_canonical_and_platform_names_use_escape_hatches() {
        assert_eq!(
            SystemIcon::shared("home").unwrap(),
            SystemIcon::Shared("home".to_owned())
        );
        assert!(SystemIcon::shared("house.fill").is_none());
        assert_eq!(
            SystemIcon::sf_symbol("house.fill").unwrap(),
            SystemIcon::SfSymbol("house.fill".to_owned())
        );
        assert_eq!(shared_icon("search").unwrap().sf_symbol, "magnifyingglass");
    }

    #[test]
    fn active_notification_icon_uses_filled_native_variants_on_both_platforms() {
        let icon = shared_icon("notifications_active").expect("shared notification icon");

        assert_eq!(icon.sf_symbol, "bell.badge.fill");
        assert_eq!(icon.material_namespace, "Filled");
        assert_eq!(icon.material_name, "NotificationsActive");
        assert!(icon.aliases.contains(&"bell.badge"));
    }

    #[test]
    fn settings_icons_match_the_filled_material_weight_on_ios() {
        for (name, sf_symbol) in [
            ("star", "star.fill"),
            ("palette", "paintpalette.fill"),
            ("notifications", "bell.fill"),
            ("email", "envelope.fill"),
            ("waving_hand", "hand.wave.fill"),
            ("document", "doc.text.fill"),
            ("security", "checkmark.shield.fill"),
            ("inbox", "tray.fill"),
            ("sun", "sun.max.fill"),
            ("settings", "gearshape.fill"),
        ] {
            let icon = shared_icon(name).expect("settings icon is in the shared catalog");
            assert_eq!(icon.sf_symbol, sf_symbol, "wrong SF Symbol for {name}");
            assert_eq!(icon.material_namespace, "Filled", "wrong style for {name}");
        }
        assert_eq!(
            shared_icon("calendar").unwrap().material_namespace,
            "Outlined"
        );
    }

    #[test]
    fn platform_specific_names_keep_native_target_spellings() {
        let sf = SystemIcon::sf_symbol("person.crop.circle.fill").unwrap();
        assert_eq!(sf.sf_symbol_name(), "person.crop.circle.fill");
        let material = SystemIcon::material_symbol("outlined:account_circle").unwrap();
        assert_eq!(
            material.material_reference(),
            "Icons.Outlined.AccountCircle"
        );
        assert_eq!(
            material.material_import(),
            "androidx.compose.material.icons.outlined.AccountCircle"
        );
        let mirrored = SystemIcon::material_symbol("autoMirroredFilled:arrow_back").unwrap();
        assert_eq!(
            mirrored.material_import(),
            "androidx.compose.material.icons.automirrored.filled.ArrowBack"
        );
    }

    #[test]
    fn widget_icons_have_android_remote_view_drawables() {
        for name in [
            "calendar_circle",
            "calendar_time",
            "check_circle",
            "circle",
            "circle_filled",
        ] {
            let icon = SystemIcon::shared(name).expect("widget icon is in shared catalog");
            let drawable = icon
                .android_widget_drawable()
                .expect("widget icon has a Glance drawable");
            assert!(drawable.resource_name.starts_with("nexa_widget_"));
            assert!(!drawable.path_data.is_empty());
        }
    }
}
