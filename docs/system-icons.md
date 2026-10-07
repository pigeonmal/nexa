# Shared System Icon Catalog

Nexa provides a unified semantic icon system. By using `Icon(system: "<name>")`, your application automatically resolves the appropriate platform-native icon at compile time:
- **iOS**: Apple SF Symbols such as `house.fill` and `magnifyingglass`.
- **Android**: Jetpack Compose Material Icons such as `Filled.Home` and `Filled.Search`.

The mapping table is maintained as the single source of truth in `crates/nexa-ir/src/system_icons.rs`.

> The catalog below is generated from `nexa_ir::system_icons::SHARED_ICONS`. Do not edit it by
> hand; run `cargo test -p nexa-ir` with `NEXA_UPDATE_SNAPSHOTS=1` to regenerate. Adding an
> icon to the catalog without updating this table fails that test.

| **Scope**: cross-platform system icons | **Targets**: SF Symbols and Material Icons | **Source**: compiler icon catalog |

## Quick start

```nx
app SearchHeader {
    body {
        Row(spacing: 8) {
            Icon(system: "search", description: "Search", size: 20, tint: "#2563EB")
            Text("Search products")
        }
    }
}
```

---

## 1. Component Usage

```nx
app SearchHeader {
    state searchSubmitted: Bool = false

    body {
        Column(spacing: 8) {
            Icon(system: "search", description: "Search", size: 24, tint: "#007AFF")
            Button("Search catalog", icon: "search") {
                searchSubmitted = true
            }
        }
    }
}
```

### Parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `system` | `String` | Exactly one icon name is required | Portable semantic name from the catalog below |
| `sfsymbol` | `String` | Exactly one icon name is required | Explicit iOS SF Symbol, for example `"sparkles"` |
| `materialsymbol` | `String` | Exactly one icon name is required | Explicit Android Material icon, for example `"auto_awesome"` |
| `description` | `String` | Required | Accessible description for the image icon; use an empty string only when it is decorative and already labeled by its parent |
| `size` | `Float64` | Required | Icon bounding-box size in platform logical units |
| `tint` | `String` | Required | Hex color or supported theme color token |

Choose exactly one of `system`, `sfsymbol`, or `materialsymbol`. Nexa does not accept both platform-specific names on one `Icon` call.

---

## 2. Platform-Specific Overrides

When a screen requires a platform-exclusive icon that does not have a cross-platform equivalent, pass explicit platform overrides:

```nx
app PlatformIcons {
    body {
        Column(spacing: 12) {
            platform ios {
                Icon(sfsymbol: "apple.logo", description: "Apple", size: 28, tint: "#1C1C1E")
            }
            platform android {
                Icon(materialsymbol: "android", description: "Android", size: 28, tint: "#1C1C1E")
            }
        }
    }
}
```

Use `platform ios { ... }` and `platform android { ... }` when an app needs to render only the matching platform-specific icon.

---

## 3. Catalog of Shared Icons

<!-- nexadoc:begin icons -->
| Shared Semantic Name | iOS SF Symbol | Android Material Icon | Native Aliases |
|---|---|---|---|
| `home` | `house.fill` | `Filled.Home` | `house`, `house.fill` |
| `search` | `magnifyingglass` | `Filled.Search` | `magnifyingglass` |
| `inbox` | `tray` | `Filled.Inbox` | `tray` |
| `person` | `person` | `Filled.Person` | `person.fill` |
| `people` | `person.2` | `Filled.Group` | `person.2.fill` |
| `favorite` | `heart` | `Outlined.FavoriteBorder` | `heart` |
| `favorite_filled` | `heart.fill` | `Filled.Favorite` | `heart.fill` |
| `comment` | `bubble.right` | `Outlined.ChatBubbleOutline` | `bubble.right` |
| `comment_left` | `bubble.left` | `Outlined.ChatBubbleOutline` | `bubble.left` |
| `comment_filled` | `bubble.right.fill` | `Filled.ChatBubble` | `bubble.left.fill`, `bubble.right.fill` |
| `bookmark` | `bookmark` | `Outlined.BookmarkBorder` | `bookmark` |
| `bookmark_filled` | `bookmark.fill` | `Filled.Bookmark` | `bookmark.fill` |
| `share` | `square.and.arrow.up` | `Filled.Share` | `arrowshape.turn.up.right` |
| `music` | `music.note` | `Filled.MusicNote` | `music.note` |
| `back` | `chevron.left` | `AutoMirrored.Filled.ArrowBack` | `chevron.left`, `arrow.left` |
| `forward` | `chevron.right` | `AutoMirrored.Filled.ArrowForward` | `chevron.right`, `arrow.right` |
| `tv` | `tv` | `Filled.Tv` | `tv` |
| `layers` | `square.3.layers.3d` | `Filled.Layers` | `rectangle.on.rectangle` |
| `add` | `plus` | `Filled.Add` | `plus` |
| `close` | `xmark` | `Filled.Close` | `xmark` |
| `check` | `checkmark` | `Filled.Check` | `checkmark` |
| `check_circle` | `checkmark.circle` | `Outlined.CheckCircleOutline` | `checkmark.circle` |
| `check_circle_filled` | `checkmark.circle.fill` | `Filled.CheckCircle` | `checkmark.circle.fill` |
| `send` | `paperplane` | `Filled.Send` | `paperplane` |
| `submit` | `arrow.up.circle.fill` | `AutoMirrored.Filled.Send` | `arrow.up.circle.fill` |
| `volume_up` | `speaker.wave.2.fill` | `Filled.VolumeUp` | `speaker.wave.2.fill` |
| `volume_off` | `speaker.slash.fill` | `AutoMirrored.Filled.VolumeOff` | `speaker.slash.fill` |
| `calendar` | `calendar` | `Filled.CalendarMonth` | `calendar` |
| `calendar_circle` | `calendar.circle` | `Filled.CalendarMonth` | `calendar.circle` |
| `event_add` | `calendar.badge.plus` | `Filled.EventAvailable` | `calendar.badge.plus` |
| `sun` | `sun.max` | `Filled.WbSunny` | `sun.max` |
| `event_done` | `calendar.badge.checkmark` | `Filled.EventAvailable` | `calendar.badge.checkmark` |
| `calendar_time` | `calendar.badge.clock` | `Filled.Event` | `calendar.badge.clock` |
| `sun_filled` | `sun.max.fill` | `Filled.WbSunny` | `sun.max.fill` |
| `sunrise` | `sunrise` | `Filled.WbTwilight` | `sunrise` |
| `settings` | `gearshape` | `Filled.Settings` | `gearshape.fill`, `gearshape` |
| `sort` | `arrow.up.arrow.down` | `Filled.Sort` | `arrow.up.arrow.down` |
| `notifications` | `bell` | `Filled.Notifications` | `bell` |
| `notifications_active` | `bell.badge.fill` | `Filled.NotificationsActive` | `bell.badge`, `bell.badge.fill` |
| `checklist` | `checklist` | `Filled.AssignmentTurnedIn` | `checklist` |
| `circle` | `circle` | `Filled.RadioButtonUnchecked` | `circle` |
| `circle_filled` | `circle.fill` | `Filled.Circle` | `circle.fill` |
| `star` | `star` | `Filled.Star` | `star` |
| `star_filled` | `star.fill` | `Filled.Star` | `star.fill` |
| `flag` | `flag` | `Filled.Flag` | `—` |
| `flag_filled` | `flag.fill` | `Filled.Flag` | `flag.fill` |
| `delete` | `trash` | `Filled.Delete` | `trash.fill`, `trash` |
| `edit` | `pencil` | `Filled.Edit` | `pencil` |
| `more` | `ellipsis` | `Filled.MoreHoriz` | `ellipsis.circle`, `ellipsis` |
| `attach` | `paperclip` | `Filled.AttachFile` | `paperclip` |
| `image` | `photo` | `Filled.Image` | `photo.fill`, `photo` |
| `camera` | `camera` | `Filled.CameraAlt` | `camera.fill`, `camera` |
| `clock` | `clock` | `Filled.Schedule` | `clock.fill`, `clock` |
| `map` | `map` | `Filled.Map` | `map.fill`, `map` |
| `location` | `location` | `Filled.LocationOn` | `location.fill`, `location` |
| `email` | `envelope` | `Filled.Email` | `envelope.fill`, `envelope` |
| `phone` | `phone` | `Filled.Call` | `phone.fill`, `phone` |
| `link` | `link` | `Filled.Link` | `link` |
| `visibility` | `eye` | `Filled.Visibility` | `eye.fill`, `eye` |
| `visibility_off` | `eye.slash` | `Filled.VisibilityOff` | `eye.slash` |
| `lock` | `lock` | `Filled.Lock` | `lock.fill`, `lock` |
| `security` | `lock.shield` | `Filled.GppGood` | `lock.shield` |
| `document` | `doc.text` | `Filled.Description` | `doc.text` |
| `language` | `globe` | `Filled.Language` | `safari` |
| `palette` | `paintpalette` | `Filled.Palette` | `paintpalette` |
| `code` | `chevron.left.forwardslash.chevron.right` | `Filled.Code` | `curlybraces` |
| `party_popper` | `party.popper` | `Filled.Celebration` | `party.popper` |
| `speedometer` | `speedometer` | `Filled.Speed` | `speedometer` |
| `waving_hand` | `hand.wave` | `Filled.WavingHand` | `hand.wave` |
| `arrow_back` | `arrow.left` | `AutoMirrored.Filled.ArrowBack` | `arrow_back` |
| `arrow_forward` | `arrow.right` | `AutoMirrored.Filled.ArrowForward` | `arrow_forward` |
| `arrow_up` | `arrow.up` | `Filled.ArrowUpward` | `arrow_upward` |
| `arrow_down` | `arrow.down` | `Filled.ArrowDownward` | `arrow_downward` |
| `menu` | `line.3.horizontal` | `Filled.Menu` | `line.3.horizontal` |
| `dashboard` | `square.grid.2x2` | `Filled.Dashboard` | `square.grid.2x2` |
| `grid` | `square.grid.3x3` | `Filled.GridView` | `square.grid.3x3` |
| `list` | `list.bullet` | `Filled.ViewList` | `list.bullet` |
| `refresh` | `arrow.clockwise` | `Filled.Refresh` | `arrow.clockwise` |
| `undo` | `arrow.uturn.backward` | `Filled.Undo` | `arrow.uturn.backward` |
| `redo` | `arrow.uturn.forward` | `Filled.Redo` | `arrow.uturn.forward` |
| `filter` | `line.3.horizontal.decrease` | `Filled.FilterList` | `line.3.horizontal.decrease` |
| `tune` | `slider.horizontal.3` | `Filled.Tune` | `slider.horizontal.3` |
| `download` | `arrow.down.to.line` | `Filled.FileDownload` | `arrow.down.to.line` |
| `upload` | `arrow.up.to.line` | `Filled.FileUpload` | `arrow.up.to.line` |
| `open` | `arrow.up.right.square` | `Filled.OpenInNew` | `arrow.up.right.square` |
| `copy` | `doc.on.doc` | `Filled.ContentCopy` | `doc.on.doc` |
| `paste` | `doc.on.clipboard` | `Filled.ContentPaste` | `doc.on.clipboard` |
| `cut` | `scissors` | `Filled.ContentCut` | `scissors` |
| `save` | `square.and.arrow.down` | `Filled.Save` | `square.and.arrow.down` |
| `print` | `printer` | `Filled.Print` | `printer` |
| `help` | `questionmark.circle` | `Outlined.HelpOutline` | `questionmark.circle` |
| `info` | `info.circle` | `Filled.Info` | `info.circle` |
| `warning` | `exclamationmark.triangle` | `Filled.Warning` | `exclamationmark.triangle` |
| `error` | `xmark.octagon` | `Filled.Error` | `xmark.octagon` |
| `verified` | `checkmark.seal` | `Filled.Verified` | `checkmark.seal` |
| `sync` | `arrow.triangle.2.circlepath` | `Filled.Sync` | `arrow.triangle.2.circlepath` |
| `cloud` | `cloud` | `Filled.Cloud` | `cloud` |
| `cloud_upload` | `icloud.and.arrow.up` | `Filled.CloudUpload` | `icloud.and.arrow.up` |
| `cloud_download` | `icloud.and.arrow.down` | `Filled.CloudDownload` | `icloud.and.arrow.down` |
| `wifi` | `wifi` | `Filled.Wifi` | `wifi` |
| `bluetooth` | `bluetooth` | `Filled.Bluetooth` | `bluetooth` |
| `battery` | `battery.100` | `Filled.BatteryFull` | `battery.100` |
| `airplane` | `airplane` | `Filled.Flight` | `airplane` |
| `flash` | `bolt` | `Filled.Bolt` | `bolt` |
| `flash_filled` | `bolt.fill` | `Filled.FlashOn` | `bolt.fill` |
| `dark_mode` | `moon` | `Filled.DarkMode` | `moon` |
| `light_mode` | `sun.max` | `Filled.LightMode` | `—` |
| `work` | `briefcase` | `Filled.Work` | `briefcase` |
| `school` | `graduationcap` | `Filled.School` | `graduationcap` |
| `home_work` | `house.and.flag` | `Filled.HomeWork` | `house.and.flag` |
| `shopping_cart` | `cart` | `Filled.ShoppingCart` | `cart` |
| `shopping_bag` | `bag` | `Filled.ShoppingBag` | `bag` |
| `payment` | `creditcard` | `Filled.CreditCard` | `creditcard` |
| `receipt` | `receipt` | `Filled.Receipt` | `receipt` |
| `wallet` | `wallet.pass` | `Filled.AccountBalanceWallet` | `wallet.pass` |
| `gift` | `gift` | `Filled.CardGiftcard` | `gift` |
| `restaurant` | `fork.knife` | `Filled.Restaurant` | `fork.knife` |
| `coffee` | `cup.and.saucer` | `Filled.Coffee` | `cup.and.saucer` |
| `fitness` | `figure.run` | `Filled.FitnessCenter` | `figure.run` |
| `health` | `heart.text.square` | `Filled.HealthAndSafety` | `heart.text.square` |
| `pets` | `pawprint` | `Filled.Pets` | `pawprint` |
| `travel` | `suitcase.rolling` | `Filled.Luggage` | `suitcase.rolling` |
| `directions` | `location.north.line` | `Filled.Navigation` | `location.north.line` |
| `place` | `mappin.and.ellipse` | `Filled.Place` | `mappin.and.ellipse` |
| `menu_book` | `book` | `Filled.MenuBook` | `book` |
| `bookmarks` | `books.vertical` | `Filled.CollectionsBookmark` | `books.vertical` |
| `folder` | `folder` | `Filled.Folder` | `folder` |
| `folder_open` | `folder` | `Filled.FolderOpen` | `folder.open` |
| `file` | `doc` | `Filled.InsertDriveFile` | `doc` |
| `pdf` | `doc.richtext` | `Filled.PictureAsPdf` | `doc.richtext` |
| `task` | `checkmark.square` | `Filled.TaskAlt` | `checkmark.square` |
| `event` | `calendar` | `Filled.Event` | `—` |
| `alarm` | `alarm` | `Filled.Alarm` | `alarm` |
| `timer` | `timer` | `Filled.Timer` | `timer` |
| `play` | `play.fill` | `Filled.PlayArrow` | `play.fill` |
| `pause` | `pause.fill` | `Filled.Pause` | `pause.fill` |
| `stop` | `stop.fill` | `Filled.Stop` | `stop.fill` |
| `skip_next` | `forward.end.fill` | `Filled.SkipNext` | `forward.end.fill` |
| `skip_previous` | `backward.end.fill` | `Filled.SkipPrevious` | `backward.end.fill` |
| `volume_down` | `speaker.wave.1.fill` | `Filled.VolumeDown` | `speaker.wave.1.fill` |
| `mute` | `speaker.slash` | `Filled.VolumeMute` | `speaker.slash` |
| `headphones` | `headphones` | `Filled.Headphones` | `headphones` |
| `microphone` | `mic` | `Filled.Mic` | `mic` |
| `microphone_off` | `mic.slash` | `Filled.MicOff` | `mic.slash` |
| `movie` | `film` | `Filled.Movie` | `film` |
| `music_library` | `music.note.list` | `Filled.LibraryMusic` | `music.note.list` |
| `chat` | `bubble.left.and.bubble.right` | `Filled.Forum` | `bubble.left.and.bubble.right` |
| `chat_filled` | `bubble.left.and.bubble.right.fill` | `Filled.Forum` | `bubble.left.and.bubble.right.fill` |
| `group_add` | `person.2.badge.plus` | `Filled.GroupAdd` | `person.2.badge.plus` |
| `account_circle` | `person.crop.circle` | `Filled.AccountCircle` | `person.crop.circle` |
| `logout` | `rectangle.portrait.and.arrow.right` | `Filled.Logout` | `rectangle.portrait.and.arrow.right` |
| `login` | `rectangle.portrait.and.arrow.left` | `Filled.Login` | `rectangle.portrait.and.arrow.left` |
| `lock_open` | `lock.open` | `Filled.LockOpen` | `lock.open` |
| `key` | `key` | `Filled.Key` | `key` |
| `shield` | `shield` | `Filled.Shield` | `shield` |
| `wifi_off` | `wifi.slash` | `Filled.WifiOff` | `wifi.slash` |
| `add_circle` | `plus.circle` | `Filled.AddCircle` | `—` |
| `remove_circle` | `minus.circle` | `Filled.RemoveCircle` | `—` |
| `cancel` | `xmark.circle` | `Filled.Cancel` | `—` |
| `expand` | `arrow.up.left.and.arrow.down.right` | `Filled.OpenInFull` | `—` |
| `collapse` | `arrow.down.right.and.arrow.up.left` | `Filled.CloseFullscreen` | `—` |
| `chevron_up` | `chevron.up` | `Filled.KeyboardArrowUp` | `—` |
| `chevron_down` | `chevron.down` | `Filled.KeyboardArrowDown` | `—` |
| `chevron_left` | `chevron.left` | `Filled.ChevronLeft` | `—` |
| `chevron_right` | `chevron.right` | `Filled.ChevronRight` | `—` |
| `open_in_new` | `arrow.up.right.square` | `Filled.OpenInNew` | `—` |
| `directions_car` | `car` | `Filled.DirectionsCar` | `—` |
| `directions_bike` | `bicycle` | `Filled.DirectionsBike` | `—` |
| `directions_walk` | `figure.walk` | `Filled.DirectionsWalk` | `—` |
| `train` | `tram` | `Filled.Train` | `—` |
| `bus` | `directions_bus` | `Filled.DirectionsBus` | `—` |
| `map_pin` | `mappin` | `Filled.Place` | `—` |
| `navigation` | `location.north.fill` | `Filled.Navigation` | `—` |
| `weather_cloud` | `cloud` | `Filled.Cloud` | `—` |
| `weather_rain` | `cloud.rain` | `Filled.WbCloudy` | `—` |
| `weather_snow` | `cloud.snow` | `Filled.AcUnit` | `—` |
| `weather_thunder` | `cloud.bolt` | `Filled.Thunderstorm` | `—` |
| `weather_night` | `moon.stars` | `Filled.NightsStay` | `—` |
| `thermostat` | `thermometer.medium` | `Filled.DeviceThermostat` | `—` |
| `water_drop` | `drop` | `Filled.WaterDrop` | `—` |
| `fire` | `flame` | `Filled.Whatshot` | `—` |
| `eco` | `leaf` | `Filled.Eco` | `—` |
| `lightbulb` | `lightbulb` | `Filled.Lightbulb` | `—` |
| `flashlight` | `flashlight.on.fill` | `Filled.FlashlightOn` | `—` |
| `computer` | `desktopcomputer` | `Filled.Computer` | `—` |
| `laptop` | `laptopcomputer` | `Filled.Laptop` | `—` |
| `tablet` | `ipad` | `Filled.TabletMac` | `—` |
| `smartphone` | `iphone` | `Filled.Smartphone` | `—` |
| `keyboard` | `keyboard` | `Filled.Keyboard` | `—` |
| `mouse` | `computermouse` | `Filled.Mouse` | `—` |
| `gamepad` | `gamecontroller` | `Filled.SportsEsports` | `—` |
| `person_add` | `person.badge.plus` | `Filled.PersonAdd` | `—` |
| `person_remove` | `person.badge.minus` | `Filled.PersonRemove` | `—` |
| `video` | `video` | `Filled.Videocam` | `—` |
| `video_off` | `video.slash` | `Filled.VideocamOff` | `—` |
| `radio` | `radio` | `Filled.Radio` | `—` |
| `equalizer` | `slider.horizontal.3` | `Filled.GraphicEq` | `—` |
| `qr_code` | `qrcode` | `Filled.QrCode` | `—` |
| `barcode` | `barcode` | `Filled.Scanner` | `—` |
| `analytics` | `chart.bar` | `Filled.BarChart` | `—` |
| `chart_pie` | `chart.pie` | `Filled.PieChart` | `—` |
| `trending_up` | `chart.line.uptrend.xyaxis` | `Filled.TrendingUp` | `—` |
| `trending_down` | `chart.line.downtrend.xyaxis` | `Filled.TrendingDown` | `—` |
| `medical` | `cross.case` | `Filled.MedicalServices` | `—` |
| `medication` | `pills` | `Filled.Medication` | `—` |
| `vaccines` | `syringe` | `Filled.Vaccines` | `—` |
| `science` | `atom` | `Filled.Science` | `—` |
| `calculate` | `function` | `Filled.Calculate` | `—` |
| `translate` | `translate` | `Filled.Translate` | `—` |
| `accessibility` | `figure.roll` | `Filled.Accessible` | `—` |
| `baby` | `figure.and.child.holdinghands` | `Filled.ChildCare` | `—` |
| `construction` | `hammer` | `Filled.Construction` | `—` |
| `apartment` | `building` | `Filled.Apartment` | `—` |
| `hotel` | `bed.double` | `Filled.Hotel` | `—` |
| `savings` | `banknote` | `Filled.Savings` | `—` |
| `account_balance` | `building.columns` | `Filled.AccountBalance` | `—` |
| `percent` | `percent` | `Filled.Percent` | `—` |
| `note` | `note.text` | `Filled.Note` | `—` |
| `sticky_note` | `note` | `Filled.StickyNote2` | `—` |
| `bookmark_add` | `bookmark.circle` | `Filled.BookmarkAdd` | `—` |
| `push_pin` | `pin` | `Filled.PushPin` | `—` |
| `history` | `clock.arrow.circlepath` | `Filled.History` | `—` |
| `snooze` | `zzz` | `Filled.Snooze` | `—` |
| `event_busy` | `calendar.badge.exclamationmark` | `Filled.EventBusy` | `—` |
| `call_end` | `phone.down` | `Filled.CallEnd` | `—` |
| `message` | `bubble.left` | `Filled.Message` | `—` |
| `notifications_off` | `bell.slash` | `Filled.NotificationsOff` | `—` |
| `privacy` | `hand.raised` | `Filled.PrivacyTip` | `—` |
| `zoom_in` | `plus.magnifyingglass` | `Filled.ZoomIn` | `—` |
| `zoom_out` | `minus.magnifyingglass` | `Filled.ZoomOut` | `—` |
<!-- nexadoc:end icons -->
