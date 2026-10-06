# Shared System Icon Catalog

Nexa provides a unified semantic icon system. By using `Icon(system: "<name>")`, your application automatically resolves the appropriate platform-native icon at compile time:
- **iOS**: Apple SF Symbols (`house.fill`, `magnifyingglass`, etc.)
- **Android**: Jetpack Compose Material Icons (`Filled.Home`, `Filled.Search`, etc.)

The mapping table is maintained as the single source of truth in `crates/nexa-ir/src/system_icons.rs`.

---

## 1. Component Usage

```nexa
// Standard semantic icon with size and color tint
Icon(system: "search", size: 24, tint: "#007AFF")

// Inside an interactive pressable button
Button(action: () => { handleSearch() }) {
    HStack(spacing: 8) {
        Icon(system: "search", size: 18)
        Text("Search catalog...")
    }
}
```

### Parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `system` | `String` | — | Canonical cross-platform semantic name from the catalog below |
| `sfsymbol` | `String` | — | Explicit iOS SF Symbol (e.g. `"sparkles"`). Overrides iOS representation |
| `materialsymbol` | `String` | — | Explicit Android Material icon (e.g. `"auto_awesome"`). Overrides Android representation |
| `size` | `Float64` | `24.0` | Icon bounding box width and height in density-independent points |
| `tint` | `String` | `"#000000"` | Hex or semantic color string (e.g. `"#007AFF"`, `"primary"`) |

---

## 2. Platform-Specific Overrides

When a screen requires a platform-exclusive icon that does not have a cross-platform equivalent, pass explicit platform overrides:

```nexa
// Uses SF Symbol on iOS, Material Symbol on Android
Icon(
    sfsymbol: "apple.logo",
    materialsymbol: "android",
    size: 28,
    tint: "#1C1C1E"
)
```

---

## 3. Catalog of Shared Icons

| Shared Semantic Name | iOS SF Symbol | Android Material Icon |
|---|---|---|
| `home` | `house.fill` | `Filled.Home` |
| `search` | `magnifyingglass` | `Filled.Search` |
| `inbox` | `tray` | `Filled.Inbox` |
| `person` | `person` | `Filled.Person` |
| `people` | `person.2` | `Filled.Group` |
| `favorite` | `heart` | `Outlined.FavoriteBorder` |
| `favorite_filled` | `heart.fill` | `Filled.Favorite` |
| `comment` | `bubble.right` | `Outlined.ChatBubbleOutline` |
| `comment_filled` | `bubble.right.fill` | `Filled.ChatBubble` |
| `bookmark` | `bookmark` | `Outlined.BookmarkBorder` |
| `bookmark_filled` | `bookmark.fill` | `Filled.Bookmark` |
| `share` | `square.and.arrow.up` | `Filled.Share` |
| `music` | `music.note` | `Filled.MusicNote` |
| `back` | `chevron.left` | `AutoMirrored.Filled.ArrowBack` |
| `forward` | `chevron.right` | `AutoMirrored.Filled.ArrowForward` |
| `tv` | `tv` | `Filled.Tv` |
| `layers` | `square.3.layers.3d` | `Filled.Layers` |
| `add` | `plus` | `Filled.Add` |
| `close` | `xmark` | `Filled.Close` |
| `check` | `checkmark` | `Filled.Check` |
| `check_circle` | `checkmark.circle` | `Outlined.CheckCircleOutline` |
| `check_circle_filled` | `checkmark.circle.fill` | `Filled.CheckCircle` |
| `send` | `paperplane` | `Filled.Send` |
| `submit` | `arrow.up.circle.fill` | `AutoMirrored.Filled.Send` |
| `volume_up` | `speaker.wave.2.fill` | `Filled.VolumeUp` |
| `volume_off` | `speaker.slash.fill` | `AutoMirrored.Filled.VolumeOff` |
| `calendar` | `calendar` | `Filled.CalendarMonth` |
| `calendar_circle` | `calendar.circle` | `Filled.CalendarMonth` |
| `event_add` | `calendar.badge.plus` | `Filled.EventAvailable` |
| `sun` | `sun.max` | `Filled.WbSunny` |
| `event_done` | `calendar.badge.checkmark` | `Filled.EventAvailable` |
| `calendar_time` | `calendar.badge.clock` | `Filled.Event` |
| `sun_filled` | `sun.max.fill` | `Filled.WbSunny` |
| `sunrise` | `sunrise` | `Filled.WbTwilight` |
| `settings` | `gearshape` | `Filled.Settings` |
| `sort` | `arrow.up.arrow.down` | `Filled.Sort` |
| `notifications` | `bell` | `Filled.Notifications` |
| `notifications_active` | `bell.badge` | `Filled.NotificationsActive` |
| `checklist` | `checklist` | `Filled.AssignmentTurnedIn` |
| `circle` | `circle` | `Filled.RadioButtonUnchecked` |
| `circle_filled` | `circle.fill` | `Filled.Circle` |
| `flag_filled` | `flag.fill` | `Filled.Flag` |
| `star` | `star` | `Filled.Star` |
| `star_filled` | `star.fill` | `Filled.Star` |
| `flag` | `flag` | `Filled.Flag` |
| `delete` | `trash` | `Filled.Delete` |
| `edit` | `pencil` | `Filled.Edit` |
| `more` | `ellipsis` | `Filled.MoreHoriz` |
| `attach` | `paperclip` | `Filled.AttachFile` |
| `image` | `photo` | `Filled.Image` |
| `camera` | `camera` | `Filled.CameraAlt` |
| `clock` | `clock` | `Filled.Schedule` |
| `map` | `map` | `Filled.Map` |
| `location` | `location` | `Filled.LocationOn` |
| `email` | `envelope` | `Filled.Email` |
| `phone` | `phone` | `Filled.Call` |
| `link` | `link` | `Filled.Link` |
| `visibility` | `eye` | `Filled.Visibility` |
| `visibility_off` | `eye.slash` | `Filled.VisibilityOff` |
| `lock` | `lock` | `Filled.Lock` |
| `security` | `lock.shield` | `Filled.GppGood` |
| `document` | `doc.text` | `Filled.Description` |
| `language` | `globe` | `Filled.Language` |
| `palette` | `paintpalette` | `Filled.Palette` |
| `code` | `chevron.left.forwardslash.chevron.right` | `Filled.Code` |
| `party_popper` | `party.popper` | `Filled.Celebration` |
| `speedometer` | `speedometer` | `Filled.Speed` |
| `waving_hand` | `hand.wave` | `Filled.WavingHand` |
| `arrow_back` | `arrow.left` | `AutoMirrored.Filled.ArrowBack` |
| `arrow_forward` | `arrow.right` | `AutoMirrored.Filled.ArrowForward` |
| `arrow_up` | `arrow.up` | `Filled.ArrowUpward` |
| `arrow_down` | `arrow.down` | `Filled.ArrowDownward` |
| `menu` | `line.3.horizontal` | `Filled.Menu` |
| `dashboard` | `square.grid.2x2` | `Filled.Dashboard` |
| `grid` | `square.grid.3x3` | `Filled.GridView` |
| `list` | `list.bullet` | `Filled.ViewList` |
| `refresh` | `arrow.clockwise` | `Filled.Refresh` |
| `undo` | `arrow.uturn.backward` | `Filled.Undo` |
| `redo` | `arrow.uturn.forward` | `Filled.Redo` |
| `filter` | `line.3.horizontal.decrease` | `Filled.FilterList` |
| `tune` | `slider.horizontal.3` | `Filled.Tune` |
| `download` | `arrow.down.to.line` | `Filled.FileDownload` |
| `upload` | `arrow.up.to.line` | `Filled.FileUpload` |
| `open` | `arrow.up.right.square` | `Filled.OpenInNew` |
| `copy` | `doc.on.doc` | `Filled.ContentCopy` |
| `paste` | `doc.on.clipboard` | `Filled.ContentPaste` |
| `cut` | `scissors` | `Filled.ContentCut` |
| `save` | `square.and.arrow.down` | `Filled.Save` |
| `print` | `printer` | `Filled.Print` |
| `help` | `questionmark.circle` | `Outlined.HelpOutline` |
| `info` | `info.circle` | `Filled.Info` |
| `warning` | `exclamationmark.triangle` | `Filled.Warning` |
| `error` | `xmark.octagon` | `Filled.Error` |
| `verified` | `checkmark.seal` | `Filled.Verified` |
| `sync` | `arrow.triangle.2.circlepath` | `Filled.Sync` |
| `cloud` | `cloud` | `Filled.Cloud` |
| `cloud_upload` | `icloud.and.arrow.up` | `Filled.CloudUpload` |
| `cloud_download` | `icloud.and.arrow.down` | `Filled.CloudDownload` |
| `wifi` | `wifi` | `Filled.Wifi` |
| `bluetooth` | `bluetooth` | `Filled.Bluetooth` |
| `battery` | `battery.100` | `Filled.BatteryFull` |
| `airplane` | `airplane` | `Filled.Flight` |
| `flash` | `bolt` | `Filled.Bolt` |
| `flash_filled` | `bolt.fill` | `Filled.FlashOn` |
| `dark_mode` | `moon` | `Filled.DarkMode` |
| `light_mode` | `sun.max` | `Filled.LightMode` |
| `work` | `briefcase` | `Filled.Work` |
| `school` | `graduationcap` | `Filled.School` |
| `home_work` | `house.and.flag` | `Filled.HomeWork` |
| `shopping_cart` | `cart` | `Filled.ShoppingCart` |
| `shopping_bag` | `bag` | `Filled.ShoppingBag` |
| `payment` | `creditcard` | `Filled.CreditCard` |
| `receipt` | `receipt` | `Filled.Receipt` |
| `wallet` | `wallet.pass` | `Filled.AccountBalanceWallet` |
| `gift` | `gift` | `Filled.CardGiftcard` |
| `restaurant` | `fork.knife` | `Filled.Restaurant` |
| `coffee` | `cup.and.saucer` | `Filled.Coffee` |
| `fitness` | `figure.run` | `Filled.FitnessCenter` |
| `health` | `heart.text.square` | `Filled.HealthAndSafety` |
| `pets` | `pawprint` | `Filled.Pets` |
| `travel` | `suitcase.rolling` | `Filled.Luggage` |
| `directions` | `location.north.line` | `Filled.Navigation` |
| `place` | `mappin.and.ellipse` | `Filled.Place` |
| `menu_book` | `book` | `Filled.MenuBook` |
| `bookmarks` | `books.vertical` | `Filled.CollectionsBookmark` |
| `folder` | `folder` | `Filled.Folder` |
| `folder_open` | `folder` | `Filled.FolderOpen` |
| `file` | `doc` | `Filled.InsertDriveFile` |
| `pdf` | `doc.richtext` | `Filled.PictureAsPdf` |
| `task` | `checkmark.square` | `Filled.TaskAlt` |
| `event` | `calendar` | `Filled.Event` |
| `alarm` | `alarm` | `Filled.Alarm` |
| `timer` | `timer` | `Filled.Timer` |
| `play` | `play.fill` | `Filled.PlayArrow` |
| `pause` | `pause.fill` | `Filled.Pause` |
| `stop` | `stop.fill` | `Filled.Stop` |
| `skip_next` | `forward.end.fill` | `Filled.SkipNext` |
| `skip_previous` | `backward.end.fill` | `Filled.SkipPrevious` |
| `volume_down` | `speaker.wave.1.fill` | `Filled.VolumeDown` |
| `mute` | `speaker.slash` | `Filled.VolumeMute` |
| `headphones` | `headphones` | `Filled.Headphones` |
| `microphone` | `mic` | `Filled.Mic` |
| `microphone_off` | `mic.slash` | `Filled.MicOff` |
| `movie` | `film` | `Filled.Movie` |
| `music_library` | `music.note.list` | `Filled.LibraryMusic` |
| `chat` | `bubble.left.and.bubble.right` | `Filled.Forum` |
| `chat_filled` | `bubble.left.and.bubble.right.fill` | `Filled.Forum` |
| `group_add` | `person.2.badge.plus` | `Filled.GroupAdd` |
| `account_circle` | `person.crop.circle` | `Filled.AccountCircle` |
| `logout` | `rectangle.portrait.and.arrow.right` | `Filled.Logout` |
| `login` | `rectangle.portrait.and.arrow.left` | `Filled.Login` |
| `lock_open` | `lock.open` | `Filled.LockOpen` |
| `key` | `key` | `Filled.Key` |
| `shield` | `shield` | `Filled.Shield` |
| `wifi_off` | `wifi.slash` | `Filled.WifiOff` |
| `add_circle` | `plus.circle` | `Filled.AddCircle` |
| `remove_circle` | `minus.circle` | `Filled.RemoveCircle` |
| `cancel` | `xmark.circle` | `Filled.Cancel` |
| `expand` | `arrow.up.left.and.arrow.down.right` | `Filled.OpenInFull` |
| `collapse` | `arrow.down.right.and.arrow.up.left` | `Filled.CloseFullscreen` |
| `chevron_up` | `chevron.up` | `Filled.KeyboardArrowUp` |
| `chevron_down` | `chevron.down` | `Filled.KeyboardArrowDown` |
| `chevron_left` | `chevron.left` | `Filled.ChevronLeft` |
| `chevron_right` | `chevron.right` | `Filled.ChevronRight` |
| `open_in_new` | `arrow.up.right.square` | `Filled.OpenInNew` |
| `directions_car` | `car` | `Filled.DirectionsCar` |
| `directions_bike` | `bicycle` | `Filled.DirectionsBike` |
| `directions_walk` | `figure.walk` | `Filled.DirectionsWalk` |
| `train` | `tram` | `Filled.Train` |
| `bus` | `directions_bus` | `Filled.DirectionsBus` |
| `map_pin` | `mappin` | `Filled.Place` |
| `navigation` | `location.north.fill` | `Filled.Navigation` |
| `weather_cloud` | `cloud` | `Filled.Cloud` |
| `weather_rain` | `cloud.rain` | `Filled.WbCloudy` |
| `weather_snow` | `cloud.snow` | `Filled.AcUnit` |
| `weather_thunder` | `cloud.bolt` | `Filled.Thunderstorm` |
| `weather_night` | `moon.stars` | `Filled.NightsStay` |
| `thermostat` | `thermometer.medium` | `Filled.DeviceThermostat` |
| `water_drop` | `drop` | `Filled.WaterDrop` |
| `fire` | `flame` | `Filled.Whatshot` |
| `eco` | `leaf` | `Filled.Eco` |
| `lightbulb` | `lightbulb` | `Filled.Lightbulb` |
| `flashlight` | `flashlight.on.fill` | `Filled.FlashlightOn` |
| `computer` | `desktopcomputer` | `Filled.Computer` |
| `laptop` | `laptopcomputer` | `Filled.Laptop` |
| `tablet` | `ipad` | `Filled.TabletMac` |
| `smartphone` | `iphone` | `Filled.Smartphone` |
| `keyboard` | `keyboard` | `Filled.Keyboard` |
| `mouse` | `computermouse` | `Filled.Mouse` |
| `gamepad` | `gamecontroller` | `Filled.SportsEsports` |
| `person_add` | `person.badge.plus` | `Filled.PersonAdd` |
| `person_remove` | `person.badge.minus` | `Filled.PersonRemove` |
| `video` | `video` | `Filled.Videocam` |
| `video_off` | `video.slash` | `Filled.VideocamOff` |
| `radio` | `radio` | `Filled.Radio` |
| `equalizer` | `slider.horizontal.3` | `Filled.GraphicEq` |
| `qr_code` | `qrcode` | `Filled.QrCode` |
| `barcode` | `barcode` | `Filled.Scanner` |
| `analytics` | `chart.bar` | `Filled.BarChart` |
| `chart_pie` | `chart.pie` | `Filled.PieChart` |
| `trending_up` | `chart.line.uptrend.xyaxis` | `Filled.TrendingUp` |
| `trending_down` | `chart.line.downtrend.xyaxis` | `Filled.TrendingDown` |
| `medical` | `cross.case` | `Filled.MedicalServices` |
| `medication` | `pills` | `Filled.Medication` |
| `vaccines` | `syringe` | `Filled.Vaccines` |
| `science` | `atom` | `Filled.Science` |
| `calculate` | `function` | `Filled.Calculate` |
| `translate` | `translate` | `Filled.Translate` |
| `accessibility` | `figure.roll` | `Filled.Accessible` |
| `baby` | `figure.and.child.holdinghands` | `Filled.ChildCare` |
| `construction` | `hammer` | `Filled.Construction` |
| `apartment` | `building` | `Filled.Apartment` |
| `hotel` | `bed.double` | `Filled.Hotel` |
| `savings` | `banknote` | `Filled.Savings` |
| `account_balance` | `building.columns` | `Filled.AccountBalance` |
| `percent` | `percent` | `Filled.Percent` |
| `note` | `note.text` | `Filled.Note` |
| `sticky_note` | `note` | `Filled.StickyNote2` |
| `bookmark_add` | `bookmark.circle` | `Filled.BookmarkAdd` |
| `push_pin` | `pin` | `Filled.PushPin` |
| `history` | `clock.arrow.circlepath` | `Filled.History` |
| `snooze` | `zzz` | `Filled.Snooze` |
| `event_busy` | `calendar.badge.exclamationmark` | `Filled.EventBusy` |
| `call_end` | `phone.down` | `Filled.CallEnd` |
| `message` | `bubble.left` | `Filled.Message` |
| `notifications_off` | `bell.slash` | `Filled.NotificationsOff` |
| `privacy` | `hand.raised` | `Filled.PrivacyTip` |
| `zoom_in` | `plus.magnifyingglass` | `Filled.ZoomIn` |
| `zoom_out` | `minus.magnifyingglass` | `Filled.ZoomOut` |
