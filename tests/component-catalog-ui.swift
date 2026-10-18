import XCTest

@MainActor
final class NexaComponentCatalogUITests: XCTestCase {
    private let catalogRoutes: [(label: String, screen: String)] = [
        ("Column spacing and padding", "ColumnCases"),
        ("Row spacing and alignment", "RowCases"),
        ("Stack overlay and alignment", "StackCases"),
        ("Spacer distribution", "SpacerCases"),
        ("Navigation title system", "TitleCases"),
        ("Divider thickness and color", "DividerCases"),
        ("Layout direction baseline", "DirectionCases"),
        ("Text styles and sizing", "TextCases"),
        ("Button styles and states", "ButtonCases"),
        ("Pressable gestures", "PressableCases"),
        ("Text input variants", "TextInputCases"),
        ("Switch states", "SwitchCases"),
        ("Slider values", "SliderCases"),
        ("Segmented control", "SegmentedCases"),
        ("Picker options", "PickerCases"),
        ("Form and section", "FormCases"),
        ("Content unavailable", "EmptyCases"),
        ("Progress bar values", "ProgressBarCases"),
        ("Progress ring values", "ProgressRingCases"),
        ("FastList row sizing", "FastListCases"),
        ("Pull to refresh", "RefreshCases"),
        ("Keyboard-aware layout", "KeyboardCases"),
        ("Icon sizes and tint", "IconCases"),
        ("Image asset rendering", "ImageCases"),
        ("Linear gradient", "GradientCases"),
        ("External link", "LinkCases"),
        ("NavigationBack behavior", "NavigationBackCases"),
        ("Toolbar placement", "ToolbarCases"),
        ("Page pager", "PagerCases"),
        ("App bottom bar", "BottomBarCases"),
        ("Adaptive split view", "SplitViewCases"),
        ("Bottom sheet", "SheetCases"),
        ("Dialog", "DialogCases"),
        ("Confirmation dialog", "ConfirmationCases"),
        ("Appearance", "AppearanceCases"),
        ("Status bar", "StatusBarCases"),
        ("Accessibility semantics", "AccessibilityCases"),
        ("Screen lifecycle", "LifecycleCases"),
    ]

    private func waitBrieflyForLayout() {
        RunLoop.current.run(until: Date().addingTimeInterval(0.25))
    }

    private func scrollTo(_ label: String, in app: XCUIApplication) throws -> XCUIElement {
        let target = app.descendants(matching: .any)
            .matching(NSPredicate(format: "label == %@", label))
            .firstMatch
        for _ in 0..<24 {
            if target.exists && target.isHittable { return target }
            app.swipeUp()
            waitBrieflyForLayout()
        }
        throw NSError(domain: "NexaComponentCatalog", code: 1, userInfo: [
            NSLocalizedDescriptionKey: "Could not reach catalog entry: \(label)",
        ])
    }

    private func scrollToTop(in app: XCUIApplication) {
        for _ in 0..<20 {
            app.swipeDown()
            waitBrieflyForLayout()
        }
    }

    private func capture(_ name: String, app: XCUIApplication) throws {
        guard let png = app.screenshot().image.pngData() else {
            throw NSError(domain: "NexaComponentCatalog", code: 2, userInfo: [
                NSLocalizedDescriptionKey: "Could not encode screen capture for \(name)",
            ])
        }
        let safeName = name.lowercased().map { $0.isLetter || $0.isNumber ? $0 : "-" }
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("nexa-component-\(String(safeName)).png")
        try png.write(to: url)

        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func navigateBack(in app: XCUIApplication, from screen: String) throws {
        let nativeBack = app.navigationBars.buttons.firstMatch
        XCTAssertTrue(
            nativeBack.waitForExistence(timeout: 8),
            "No native back control appeared for \(screen)"
        )
        XCTAssertTrue(nativeBack.isHittable, "Native back control is not hittable for \(screen)")
        nativeBack.tap()
        waitBrieflyForLayout()
    }

    private func push(_ label: String, screen: String, in app: XCUIApplication) throws {
        let link = try scrollTo(label, in: app)
        link.tap()

        let nativeBack = app.navigationBars.buttons.firstMatch
        XCTAssertTrue(
            nativeBack.waitForExistence(timeout: 10),
            "Navigation did not open \(screen) from \(label)"
        )
        waitBrieflyForLayout()
        // AppBottomBar presents the selected tab's native navigation title.
        // That title is intentionally "Inbox", not the fixture route name.
        let navigationTitle = screen == "BottomBarCases" ? "Inbox" : screen
        XCTAssertTrue(
            app.staticTexts[navigationTitle].waitForExistence(timeout: 5),
            "The native navigation title did not identify \(screen) (expected \(navigationTitle))"
        )
        XCTAssertTrue(app.staticTexts.count > 0, "\(screen) rendered no accessible text")
        try capture(screen, app: app)
    }

    private func tap(_ label: String, in app: XCUIApplication) throws {
        let target = app.descendants(matching: .any)
            .matching(NSPredicate(format: "label == %@", label))
            .firstMatch
        XCTAssertTrue(target.waitForExistence(timeout: 8), "Missing navigation control: \(label)")
        XCTAssertTrue(target.isHittable, "Navigation control is not hittable: \(label)")
        target.tap()
        waitBrieflyForLayout()
    }

    private func open(_ label: String, screen: String, in app: XCUIApplication) throws {
        try push(label, screen: screen, in: app)
        try navigateBack(in: app, from: screen)
    }

    func testEveryComponentScreenOpensAndReturns() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(
            app.staticTexts["Column spacing and padding"].waitForExistence(timeout: 30),
            "Component catalog did not appear"
        )
        for route in catalogRoutes {
            try open(route.label, screen: route.screen, in: app)
        }

        scrollToTop(in: app)
        try push("Navigation title system", screen: "TitleCases", in: app)
        try push("Title with a leading toolbar item", screen: "TitleLeadingCases", in: app)
        try navigateBack(in: app, from: "TitleLeadingCases")
        try push("Title without toolbar items", screen: "TitleNoToolbarCases", in: app)
        try navigateBack(in: app, from: "TitleNoToolbarCases")
        try navigateBack(in: app, from: "TitleCases")
    }

    func testNestedScreenTransitionsPushAndPop() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(
            app.staticTexts["Column spacing and padding"].waitForExistence(timeout: 30),
            "Component catalog did not appear"
        )
        try push("NavigationBack behavior", screen: "NavigationBackCases", in: app)

        try tap("Conditional navigation link", in: app)
        XCTAssertTrue(
            app.staticTexts["DirectionCases"].waitForExistence(timeout: 8),
            "The first navigation push did not show DirectionCases"
        )
        try capture("NavigationTransitionPush", app: app)

        try tap("Open nested navigation level", in: app)
        XCTAssertTrue(
            app.staticTexts["NavigationBackCases"].waitForExistence(timeout: 8),
            "The nested navigation push did not show NavigationBackCases"
        )
        try capture("NavigationTransitionNestedPush", app: app)

        try navigateBack(in: app, from: "nested NavigationBackCases")
        XCTAssertTrue(
            app.buttons["Open nested navigation level"].waitForExistence(timeout: 5),
            "The first pop did not restore DirectionCases"
        )
        try capture("NavigationTransitionFirstPop", app: app)

        try navigateBack(in: app, from: "DirectionCases")
        XCTAssertTrue(
            app.buttons["Conditional navigation link"].waitForExistence(timeout: 5),
            "The second pop did not restore the original NavigationBackCases screen"
        )
        try capture("NavigationTransitionRootRestored", app: app)
    }

    func testTitleToolbarAndContextMenuActions() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(
            app.staticTexts["Column spacing and padding"].waitForExistence(timeout: 30),
            "Component catalog did not appear"
        )
        scrollToTop(in: app)
        try push("Navigation title system", screen: "TitleCases", in: app)

        let moreActions = app.descendants(matching: .any)
            .matching(NSPredicate(format: "label == %@", "More actions"))
            .firstMatch
        XCTAssertTrue(moreActions.waitForExistence(timeout: 5), "Title toolbar menu trigger is missing")
        moreActions.tap()
        XCTAssertTrue(app.staticTexts["Action count: 1"].waitForExistence(timeout: 5))

        moreActions.press(forDuration: 0.8)
        let archive = app.buttons["Archive"]
        XCTAssertTrue(archive.waitForExistence(timeout: 5), "Long press did not open the Archive menu item")
        try capture("TitleMenuOpen", app: app)
        archive.tap()
        XCTAssertTrue(app.staticTexts["Action count: 11"].waitForExistence(timeout: 5))

        moreActions.press(forDuration: 0.8)
        let delete = app.buttons["Delete"]
        XCTAssertTrue(delete.waitForExistence(timeout: 5), "Long press did not reopen the Delete menu item")
        delete.tap()
        XCTAssertTrue(app.staticTexts["Action count: 111"].waitForExistence(timeout: 5))
    }

    func testBottomBarUsesSelectedTabAsNativeTitle() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(
            app.staticTexts["Column spacing and padding"].waitForExistence(timeout: 30),
            "Component catalog did not appear"
        )
        try push("App bottom bar", screen: "BottomBarCases", in: app)
        XCTAssertTrue(app.staticTexts["Inbox tab content"].waitForExistence(timeout: 5))
        try navigateBack(in: app, from: "BottomBarCases")
    }

    func testTextStyleTabsSwitchOnFirstTap() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(
            app.staticTexts["Column spacing and padding"].waitForExistence(timeout: 30),
            "Component catalog did not appear"
        )
        try push("Text styles and sizing", screen: "TextCases", in: app)

        let layoutTab = app.buttons["Text layout and effects tests"]
        XCTAssertTrue(layoutTab.waitForExistence(timeout: 5), "Layout tab is missing")
        layoutTab.tap()
        let leadingText = app.descendants(matching: .any)
            .matching(NSPredicate(format: "label CONTAINS %@", "Leading alignment"))
            .firstMatch
        XCTAssertTrue(
            leadingText.waitForExistence(timeout: 5),
            "Layout content did not switch after one tap"
        )
        try capture("TextLayoutFirstTap", app: app)

        let fontStylesTab = app.buttons["Text semantic font styles"]
        XCTAssertTrue(fontStylesTab.waitForExistence(timeout: 5), "Font styles tab is missing")
        fontStylesTab.tap()
        XCTAssertTrue(
            app.staticTexts["LargeTitle"].waitForExistence(timeout: 5),
            "Font style content did not switch after one tap"
        )
        try capture("TextFontStylesFirstTap", app: app)

        let typographyTab = app.buttons["Text typography tests"]
        XCTAssertTrue(typographyTab.waitForExistence(timeout: 5), "Typography tab is missing")
        typographyTab.tap()
        XCTAssertTrue(
            app.staticTexts["Normal 14"].waitForExistence(timeout: 5),
            "Typography content did not switch after one tap"
        )
        try capture("TextTypographyFirstTap", app: app)
    }

    func testDividerColorAndThicknessCasesRender() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(
            app.staticTexts["Column spacing and padding"].waitForExistence(timeout: 30),
            "Component catalog did not appear"
        )
        try push("Divider thickness and color", screen: "DividerCases", in: app)

        for label in ["Gray · 1 dp", "Red · 2 dp", "Blue · 4 dp"] {
            XCTAssertTrue(app.staticTexts[label].exists, "Divider case is missing: \(label)")
        }
        for label in ["DividerCases", "Divider", "Color and thickness", "Gray · 1 dp", "Red · 2 dp", "Blue · 4 dp"] {
            let element = app.descendants(matching: .any)
                .matching(NSPredicate(format: "label == %@", label))
                .firstMatch
            print("DIVIDER_FRAME \(label): \(element.frame)")
        }
        try capture("DividerColorAndThickness", app: app)
    }

    func testSwitchBoundValueTogglesOnAndOff() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(
            app.staticTexts["Column spacing and padding"].waitForExistence(timeout: 30),
            "Component catalog did not appear"
        )
        try push("Switch states", screen: "SwitchCases", in: app)

        let notifications = app.switches["Notifications"]
        XCTAssertTrue(notifications.waitForExistence(timeout: 5), "Notifications switch is missing")
        XCTAssertEqual(notifications.value as? String, "0", "Switch should start off")
        notifications.tap()
        waitBrieflyForLayout()
        XCTAssertEqual(notifications.value as? String, "1", "First tap should turn the switch on")
        try capture("SwitchEnabled", app: app)

        notifications.tap()
        waitBrieflyForLayout()
        XCTAssertEqual(notifications.value as? String, "0", "Second tap should restore the switch off state")
        try capture("SwitchRestored", app: app)
    }

    func testFastListReverseLayoutKeepsScrollPositionInSync() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(
            app.staticTexts["Column spacing and padding"].waitForExistence(timeout: 30),
            "Component catalog did not appear"
        )
        try push("FastList row sizing", screen: "FastListCases", in: app)

        let reversePage = app.buttons["Page 2"]
        XCTAssertTrue(reversePage.waitForExistence(timeout: 5), "Reverse-layout page is missing")
        reversePage.tap()

        let position = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH %@", "Scroll position:")
        ).firstMatch
        XCTAssertTrue(position.waitForExistence(timeout: 5), "Bound scroll position is missing")
        XCTAssertEqual(position.label, "Scroll position: 0", "Reverse list should start at logical row zero")

        let list = app.tables.firstMatch
        XCTAssertTrue(list.waitForExistence(timeout: 5), "FastList did not expose its native scrolling surface")
        waitBrieflyForLayout()
        list.swipeUp()

        let changedPosition = NSPredicate(format: "label != %@", "Scroll position: 0")
        expectation(for: changedPosition, evaluatedWith: position)
        waitForExpectations(timeout: 8)
        let updatedPosition = Int(position.label.replacingOccurrences(of: "Scroll position: ", with: ""))
        XCTAssertNotNil(updatedPosition, "Scroll position label was malformed: \(position.label)")
        XCTAssertGreaterThan(updatedPosition ?? 0, 0, "Reverse list swipe did not update its bound position")
        XCTAssertTrue(
            app.staticTexts["Reverse row \(updatedPosition ?? 0)"].exists,
            "Visible logical row should follow the bound scroll position"
        )
        try capture("FastListReverseScrollPosition", app: app)
    }

    func testSegmentedControlSelectionUpdatesOnFirstTap() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(
            app.staticTexts["Column spacing and padding"].waitForExistence(timeout: 30),
            "Component catalog did not appear"
        )
        try push("Segmented control", screen: "SegmentedCases", in: app)

        let controls = [
            ("Default time range", "Default"),
            ("Spring time range", "Spring"),
            ("Tween time range", "Tween"),
        ]
        for (accessibilityLabel, prefix) in controls {
            let segments = app.segmentedControls.matching(
                NSPredicate(format: "label == %@", accessibilityLabel)
            ).firstMatch
            XCTAssertTrue(segments.waitForExistence(timeout: 5), "Missing segmented control: \(accessibilityLabel)")
            for selection in ["Week", "Month", "Day"] {
                let button = segments.buttons[selection]
                XCTAssertTrue(button.exists, "Segment is missing from \(accessibilityLabel): \(selection)")
                button.tap()
                XCTAssertTrue(
                    app.staticTexts["\(prefix): \(selection)"].waitForExistence(timeout: 5),
                    "\(accessibilityLabel) selection did not update after one tap: \(selection)"
                )
                try capture("Segmented\(prefix)\(selection)", app: app)
            }
        }

        XCTAssertTrue(app.staticTexts["Single selection: Only"].exists)
        XCTAssertTrue(app.staticTexts["Empty selection: Missing"].exists)
        XCTAssertTrue(app.staticTexts["Missing selection: Year"].exists)
    }

    func testPickerSelectionUpdatesOnFirstTap() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(
            app.staticTexts["Column spacing and padding"].waitForExistence(timeout: 30),
            "Component catalog did not appear"
        )
        try push("Picker options", screen: "PickerCases", in: app)
        XCTAssertTrue(app.staticTexts["Selected: Due date"].exists)

        for selection in ["Priority", "Title", "Due date"] {
            let trigger = app.buttons.matching(
                NSPredicate(format: "label CONTAINS[c] %@", "Sort tasks")
            ).firstMatch
            XCTAssertTrue(trigger.waitForExistence(timeout: 5), "Sort picker trigger is missing")
            trigger.tap()
            if selection == "Priority" {
                try capture("PickerMenuOpen", app: app)
            }

            try tapPickerOption(selection, in: app)
            XCTAssertTrue(
                app.staticTexts["Selected: \(selection)"].waitForExistence(timeout: 5),
                "Picker selection did not update after one tap: \(selection)"
            )
            try capture("Picker\(selection.replacingOccurrences(of: " ", with: ""))", app: app)
        }

        let rowIconTrigger = app.buttons.matching(
            NSPredicate(format: "label CONTAINS[c] %@", "Sort by")
        ).firstMatch
        XCTAssertTrue(rowIconTrigger.waitForExistence(timeout: 5), "Per-row icon picker trigger is missing")
        for selection in ["Due date", "Priority", "Title"] {
            rowIconTrigger.tap()
            if selection == "Due date" {
                XCTAssertTrue(app.buttons["Due date"].waitForExistence(timeout: 5))
                try capture("PickerRowIconsMenu", app: app)
            }
            try tapPickerOption(selection, in: app)
            XCTAssertTrue(
                app.staticTexts["Selected row icon: \(selection)"].waitForExistence(timeout: 5),
                "Per-row icon picker did not select \(selection) on first tap"
            )
            try capture("PickerRowIcon\(selection.replacingOccurrences(of: " ", with: ""))", app: app)
        }

        let labelOnlyTrigger = app.buttons.matching(
            NSPredicate(format: "label CONTAINS[c] %@", "Category")
        ).firstMatch
        XCTAssertTrue(labelOnlyTrigger.waitForExistence(timeout: 5), "Label-only picker trigger is missing")
        labelOnlyTrigger.tap()
        try tapPickerOption("Title", in: app)
        XCTAssertTrue(
            app.staticTexts["Selected label-only: Title"].waitForExistence(timeout: 5),
            "Label-only picker selection did not update after one tap"
        )
        try capture("PickerLabelOnly", app: app)

        let iconOnlyTrigger = app.buttons["Choose an option"]
        XCTAssertTrue(iconOnlyTrigger.waitForExistence(timeout: 5), "Icon-only picker trigger is missing")
        iconOnlyTrigger.tap()
        try tapPickerOption("Due date", in: app)
        XCTAssertTrue(
            app.staticTexts["Selected icon-only: Due date"].waitForExistence(timeout: 5),
            "Icon-only picker selection did not update after one tap"
        )
        try capture("PickerIconOnly", app: app)

        let defaultTrigger = app.descendants(matching: .any).matching(
            NSPredicate(format: "label == %@", "Due date")
        ).firstMatch
        XCTAssertTrue(defaultTrigger.waitForExistence(timeout: 5), "Default picker trigger is missing")
        defaultTrigger.tap()
        try tapPickerOption("Priority", in: app)
        XCTAssertTrue(
            app.staticTexts["Selected default: Priority"].waitForExistence(timeout: 5),
            "Default picker selection did not update after one tap"
        )
        try capture("PickerDefaults", app: app)
    }
    private func tapSwitchThumb(_ element: XCUIElement) {
        element.coordinate(withNormalizedOffset: CGVector(dx: 0.92, dy: 0.5)).tap()
    }

    private func tapPickerOption(_ label: String, in app: XCUIApplication) throws {
        let matches = app.buttons.matching(NSPredicate(format: "label == %@", label))
        let deadline = Date().addingTimeInterval(5)
        var option = matches.allElementsBoundByIndex.first(where: \.isHittable)
        while option == nil && Date() < deadline {
            RunLoop.current.run(until: Date().addingTimeInterval(0.1))
            option = matches.allElementsBoundByIndex.first(where: \.isHittable)
        }
        XCTAssertNotNil(option, "Picker option is missing or not hittable: \(label)")
        try XCTUnwrap(option).tap()
    }

}
