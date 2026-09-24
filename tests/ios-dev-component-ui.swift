import XCTest

final class NexaDevComponentTests: XCTestCase {
    private func assertText(_ fragment: String, in app: XCUIApplication, file: StaticString = #filePath, line: UInt = #line) {
        let match = app.staticTexts.matching(NSPredicate(format: "label CONTAINS %@", fragment)).firstMatch
        XCTAssertTrue(match.exists, "Missing rendered text containing: \(fragment)", file: file, line: line)
    }

    private func assertAccessibleLabel(_ label: String, in app: XCUIApplication, file: StaticString = #filePath, line: UInt = #line) {
        let match = app.descendants(matching: .any).matching(NSPredicate(format: "label == %@", label)).firstMatch
        XCTAssertTrue(match.exists, "Missing accessible label: \(label)", file: file, line: line)
    }

    func testReloadedComponentArgumentsAndProjectedContentRender() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Card: Reloaded"].waitForExistence(timeout: 45))
        assertText("Projected child", in: app)
        assertText("Binary and: true", in: app)
        assertText("Contains: true", in: app)
        assertText("Index: 5", in: app)
        assertText("Coalesce: fallback", in: app)
        assertText("Pair: 7, Mapped: 6, Filtered: 8, Reduced: 16", in: app)
        assertText("Int8: 8, Int16: 16, Int64: 64", in: app)
        assertText("UInt8: 8, UInt16: 16, UInt32: 32, UInt64: 64", in: app)
        assertText("Float32: 3.5, Float64: 6.5", in: app)
        assertText("Set: true, Map: 42, Triple: 4, Enum: ready, Result: 42, Result error: null, Struct: Ada", in: app)
        assertText("Array mutation: 2, Set mutation: true, Map mutation: 2", in: app)
        assertText("For: 6, While: 3, Break: 2, Continue: 5, ForMap: 2, TryCatch: 1", in: app)
        assertText("Other comparisons: true, true, true, true, true, true", in: app)
        assertText("Size classes:", in: app)
        assertText("Native path:", in: app)

        for label in ["Static style", "Adaptive style", "Semibold style", "Bold style", "Styled start", "Styled center", "Styled end", "Ease in out", "Linear", "Never dismiss"] {
            assertText(label, in: app)
        }

        for label in ["Link role", "Header role", "Image role", "No role"] {
            assertAccessibleLabel(label, in: app)
        }
        let externalLink = app.descendants(matching: .any)
            .matching(NSPredicate(format: "label CONTAINS %@", "External link"))
            .firstMatch
        XCTAssertTrue(externalLink.exists, "Missing external link node: \(app.debugDescription)")

        for placeholder in ["Keyboard text", "Keyboard number", "Keyboard email", "Keyboard phone", "Keyboard url"] {
            XCTAssertTrue(app.textFields[placeholder].exists, "Missing text field: \(placeholder)")
        }

        for label in ["Haptic light", "Haptic medium", "Haptic heavy"] {
            let button = app.buttons[label]
            XCTAssertTrue(button.exists, "Missing \(label)")
            button.tap()
        }
    }
}
