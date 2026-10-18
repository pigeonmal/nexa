struct NexaDatePickerPrimitive: View {
    private let timestamp: Binding<Int64>?
    private let startTimestamp: Binding<Int64>?
    private let endTimestamp: Binding<Int64>?
    private let pickerType: String
    private let style: String
    private let selection: String
    private let minimum: Int64?
    private let maximum: Int64?
    private let tint: Color?
    private let textColor: Color?
    private let backgroundColor: Color?
    private let pickerBackgroundColor: Color?
    private let borderColor: Color?
    private let borderWidth: CGFloat?
    private let cornerRadius: CGFloat?
    private let localeIdentifier: String?
    private let timeZoneIdentifier: String?
    private let is24Hour: Bool?
    private let enabled: Bool?
    private let accessibilityLabel: String?

    @State private var localTimestamp: Int64
    @State private var localStartTimestamp: Int64
    @State private var localEndTimestamp: Int64
    @State private var isRangePresented = false
    @State private var rangeDraft = Set<DateComponents>()
    @State private var rangeAnchor: Date?
    @State private var rangeHasStarted = false

    init(
        timestamp: Binding<Int64>?,
        startTimestamp: Binding<Int64>?,
        endTimestamp: Binding<Int64>?,
        pickerType: String,
        style: String,
        selection: String,
        minimum: Int64?,
        maximum: Int64?,
        tint: Color?,
        textColor: Color?,
        backgroundColor: Color?,
        pickerBackgroundColor: Color?,
        borderColor: Color?,
        borderWidth: CGFloat?,
        cornerRadius: CGFloat?,
        localeIdentifier: String?,
        timeZoneIdentifier: String?,
        is24Hour: Bool?,
        enabled: Bool?,
        accessibilityLabel: String?
    ) {
        self.timestamp = timestamp
        self.startTimestamp = startTimestamp
        self.endTimestamp = endTimestamp
        self.pickerType = pickerType
        self.style = style
        self.selection = selection
        self.minimum = minimum
        self.maximum = maximum
        self.tint = tint
        self.textColor = textColor
        self.backgroundColor = backgroundColor
        self.pickerBackgroundColor = pickerBackgroundColor
        self.borderColor = borderColor
        self.borderWidth = borderWidth
        self.cornerRadius = cornerRadius
        self.localeIdentifier = localeIdentifier
        self.timeZoneIdentifier = timeZoneIdentifier
        self.is24Hour = is24Hour
        self.enabled = enabled
        self.accessibilityLabel = accessibilityLabel

        let now = Int64(Date().timeIntervalSince1970 * 1_000)
        _localTimestamp = State(initialValue: timestamp?.wrappedValue ?? now)
        _localStartTimestamp = State(initialValue: startTimestamp?.wrappedValue ?? now)
        _localEndTimestamp = State(initialValue: endTimestamp?.wrappedValue ?? now)
    }

    var body: some View {
        let content = pickerContent
            .environment(\.locale, pickerLocale)
            .environment(\.timeZone, pickerTimeZone)
            .environment(\.calendar, pickerCalendar)
            .modifier(NexaDatePickerAppearance(
                backgroundColor: backgroundColor,
                pickerBackgroundColor: pickerBackgroundColor,
                borderColor: borderColor,
                borderWidth: borderWidth,
                cornerRadius: cornerRadius
            ))
            .tint(tint)
            .foregroundStyle(textColor ?? .primary)
            .disabled(!pickerIsEnabled)

        if let accessibilityLabel {
            content.accessibilityLabel(accessibilityLabel)
        } else {
            content
        }
    }

    @ViewBuilder
    private var pickerContent: some View {
        if selection == "Range" {
            rangePicker
        } else {
            singlePicker
        }
    }

    @ViewBuilder
    private var singlePicker: some View {
        let picker = DatePicker("", selection: selectedDate, in: minimumDate...maximumDate, displayedComponents: displayedComponents)
            .labelsHidden()

        switch style {
        case "Graphical":
            picker.datePickerStyle(.graphical)
        case "Wheel":
            picker.datePickerStyle(.wheel)
        case "Compact", "Input":
            picker.datePickerStyle(.compact)
        default:
            picker.datePickerStyle(.automatic)
        }
    }

    @ViewBuilder
    private var rangePicker: some View {
        if style == "Compact" || style == "Input" {
            Button {
                beginRangeSelection()
            } label: {
                Text(rangeSummary)
                    .font(.system(size: 14, weight: .medium))
                    .lineLimit(1)
                    .minimumScaleFactor(0.8)
                    .padding(.horizontal, 16)
                    .frame(minHeight: 40)
                    .overlay {
                        Capsule()
                            .stroke(Color.secondary.opacity(0.38), lineWidth: 1)
                    }
                    .contentShape(Capsule())
            }
            .buttonStyle(.plain)
            .accessibilityValue(rangeSummary)
            .sheet(isPresented: $isRangePresented) {
                rangeSelectionSheet
            }
            .accessibilityHint("Opens the date range picker")
            .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
        } else {
            inlineRangeCalendar
        }
    }

    private var rangeCalendar: some View {
        MultiDatePicker("", selection: rangeDraftBinding, in: rangeDateRange)
            .labelsHidden()
            .datePickerStyle(.graphical)
    }

    private var inlineRangeCalendar: some View {
        MultiDatePicker("", selection: selectedRange, in: rangeDateRange)
            .labelsHidden()
            .datePickerStyle(.graphical)
    }

    private var rangeSelectionSheet: some View {
        NavigationStack {
            VStack {
                rangeCalendar
            }
            .padding(.horizontal, 16)
            .padding(.top, 24)
            .navigationTitle("Select date range")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        isRangePresented = false
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") {
                        commitRangeSelection()
                    }
                    .disabled(rangeDraft.count < 2)
                }
            }
        }
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
    }

    private var dateSummaryStyle: Date.FormatStyle {
        var style = Date.FormatStyle(date: .abbreviated, time: .omitted)
        style.locale = pickerLocale
        style.timeZone = pickerTimeZone
        style.calendar = pickerCalendar
        return style
    }

    private var rangeDraftBinding: Binding<Set<DateComponents>> {
        Binding(
            get: { rangeDraft },
            set: updateRangeDraft
        )
    }

    private var currentStartTimestamp: Int64 {
        constrainedTimestamp(startTimestamp?.wrappedValue ?? localStartTimestamp)
    }

    private var currentEndTimestamp: Int64 {
        constrainedTimestamp(endTimestamp?.wrappedValue ?? localEndTimestamp)
    }

    private func beginRangeSelection() {
        rangeDraft = Set([currentStartTimestamp, currentEndTimestamp].map { value in
            pickerCalendar.dateComponents([.year, .month, .day], from: date(for: value))
        })
        rangeAnchor = nil
        rangeHasStarted = false
        isRangePresented = true
    }

    private func updateRangeDraft(_ selection: Set<DateComponents>) {
        let selectedDays = Set(
            selection
                .compactMap(pickerCalendar.date(from:))
                .map { pickerCalendar.startOfDay(for: $0) }
        )
        let previousDays = Set(
            rangeDraft
                .compactMap(pickerCalendar.date(from:))
                .map { pickerCalendar.startOfDay(for: $0) }
        )
        let changedDates = selectedDays.symmetricDifference(previousDays).sorted()
        guard let changedDate = changedDates.last else {
            rangeDraft = selection
            return
        }
        let changed = pickerCalendar.startOfDay(for: changedDate)

        if !rangeHasStarted {
            rangeHasStarted = true
            rangeAnchor = changed
            rangeDraft = [pickerCalendar.dateComponents([.year, .month, .day], from: changed)]
            return
        }

        guard let anchor = rangeAnchor else {
            rangeAnchor = changed
            rangeDraft = [pickerCalendar.dateComponents([.year, .month, .day], from: changed)]
            return
        }

        if selectedDays.contains(changed), changed != anchor {
            rangeDraft = Set([anchor, changed].map {
                pickerCalendar.dateComponents([.year, .month, .day], from: $0)
            })
        } else {
            rangeAnchor = changed
            rangeDraft = [pickerCalendar.dateComponents([.year, .month, .day], from: changed)]
        }
    }

    private func commitRangeSelection() {
        let dates = rangeDraft.compactMap(pickerCalendar.date(from:)).sorted()
        guard let first = dates.first, let last = dates.last, dates.count > 1 else { return }
        let start = timestamp(for: first)
        let end = timestamp(for: last)
        localStartTimestamp = start
        localEndTimestamp = end
        startTimestamp?.wrappedValue = start
        endTimestamp?.wrappedValue = end
        isRangePresented = false
    }

    private var selectedDate: Binding<Date> {
        Binding(
            get: { date(for: constrainedTimestamp(currentTimestamp)) },
            set: { newValue in
                let value = constrainedTimestamp(timestamp(for: canonicalDate(newValue)))
                localTimestamp = value
                timestamp?.wrappedValue = value
            }
        )
    }

    private var selectedRange: Binding<Set<DateComponents>> {
        Binding(
            get: {
                Set([currentStartTimestamp, currentEndTimestamp].map { value in
                    pickerCalendar.dateComponents([.year, .month, .day], from: date(for: value))
                })
            },
            set: { components in
                let dates = components
                    .compactMap { pickerCalendar.date(from: $0) }
                    .sorted()
                guard let first = dates.first else { return }
                let start = timestamp(for: first)
                localStartTimestamp = start
                if dates.count > 1, let last = dates.last {
                    let end = timestamp(for: last)
                    localEndTimestamp = end
                    startTimestamp?.wrappedValue = start
                    endTimestamp?.wrappedValue = end
                } else {
                    localEndTimestamp = start
                }
            }
        )
    }

    private var displayedComponents: DatePickerComponents {
        switch pickerType {
        case "Time": [.hourAndMinute]
        case "DateTime": [.date, .hourAndMinute]
        default: [.date]
        }
    }

    private var currentTimestamp: Int64 {
        timestamp?.wrappedValue ?? localTimestamp
    }

    private var minimumDate: Date {
        guard boundsAreValid, let minimum else { return .distantPast }
        switch pickerType {
        case "Date": return pickerCalendar.startOfDay(for: date(for: minimum))
        case "Time": return selectableTimeRange?.lowerBound ?? .distantPast
        default: return ceilToMinute(date(for: minimum))
        }
    }

    private var maximumDate: Date {
        guard boundsAreValid, let maximum else { return .distantFuture }
        switch pickerType {
        case "Date": return pickerCalendar.startOfDay(for: date(for: maximum))
        case "Time": return selectableTimeRange?.upperBound ?? .distantFuture
        default: return floorToMinute(date(for: maximum))
        }
    }

    private var rangeDateRange: Range<Date> {
        guard boundsAreValid else { return Date.distantPast..<Date.distantFuture }
        let lowerBound = minimum.map { pickerCalendar.startOfDay(for: date(for: $0)) } ?? .distantPast
        let upperBound = maximum.flatMap { value in
            pickerCalendar.date(byAdding: .day, value: 1, to: pickerCalendar.startOfDay(for: date(for: value)))
        } ?? .distantFuture
        return lowerBound..<upperBound
    }

    private var pickerTimeZone: TimeZone {
        timeZoneIdentifier.flatMap(TimeZone.init(identifier:)) ?? .current
    }

    private var pickerLocale: Locale {
        var components = Locale.Components(identifier: localeIdentifier ?? Locale.current.identifier)
        if let is24Hour {
            components.hourCycle = is24Hour ? .zeroToTwentyThree : .oneToTwelve
        }
        return Locale(components: components)
    }

    private var pickerCalendar: Calendar {
        var calendar = Calendar(identifier: pickerLocale.calendar.identifier)
        calendar.locale = pickerLocale
        calendar.timeZone = pickerTimeZone
        return calendar
    }

    private var boundsAreValid: Bool {
        guard let minimum, let maximum else { return true }
        if pickerType == "Date" || selection == "Range" {
            return pickerCalendar.startOfDay(for: date(for: minimum)) <=
                pickerCalendar.startOfDay(for: date(for: maximum))
        }
        return minimum <= maximum
    }

    private var pickerIsEnabled: Bool {
        guard enabled != false, boundsAreValid else { return false }
        if pickerType == "Time" { return selectableTimeRange != nil }
        if pickerType == "DateTime", let minimum, let maximum {
            return ceilToMinute(date(for: minimum)) <= floorToMinute(date(for: maximum))
        }
        return true
    }

    private var selectableTimeRange: ClosedRange<Date>? {
        guard let day = pickerCalendar.dateInterval(of: .day, for: date(for: currentTimestamp)) else { return nil }
        let lower = max(minimum.map { date(for: $0) } ?? day.start, day.start)
        let upper = min(maximum.map { date(for: $0) } ?? day.end.addingTimeInterval(-0.001), day.end.addingTimeInterval(-0.001))
        let firstMinute = ceilToMinute(lower)
        let lastMinute = floorToMinute(upper)
        guard firstMinute <= lastMinute else { return nil }
        return firstMinute...lastMinute
    }

    private func canonicalDate(_ proposed: Date) -> Date {
        switch pickerType {
        case "Date":
            return pickerCalendar.startOfDay(for: proposed)
        case "Time":
            let components = pickerCalendar.dateComponents([.hour, .minute], from: proposed)
            return pickerCalendar.date(
                bySettingHour: components.hour ?? 0,
                minute: components.minute ?? 0,
                second: 0,
                of: date(for: currentTimestamp)
            ) ?? date(for: currentTimestamp)
        default:
            return floorToMinute(proposed)
        }
    }

    private func constrainedTimestamp(_ timestamp: Int64) -> Int64 {
        guard boundsAreValid else { return timestamp }
        let proposed = date(for: timestamp)
        switch pickerType {
        case "Date":
            var selected = pickerCalendar.startOfDay(for: proposed)
            let lower = minimum.map { pickerCalendar.startOfDay(for: date(for: $0)) } ?? .distantPast
            let upper = maximum.map { pickerCalendar.startOfDay(for: date(for: $0)) } ?? .distantFuture
            selected = min(max(selected, lower), upper)
            return self.timestamp(for: selected)
        case "Time":
            guard let selectableTimeRange else { return timestamp }
            let selected = min(max(proposed, selectableTimeRange.lowerBound), selectableTimeRange.upperBound)
            return self.timestamp(for: selected)
        default:
            let lower = minimum.map { ceilToMinute(date(for: $0)) } ?? .distantPast
            let upper = maximum.map { floorToMinute(date(for: $0)) } ?? .distantFuture
            guard lower <= upper else { return timestamp }
            let selected = min(max(floorToMinute(proposed), lower), upper)
            return self.timestamp(for: selected)
        }
    }

    private func floorToMinute(_ date: Date) -> Date {
        pickerCalendar.dateInterval(of: .minute, for: date)?.start ?? date
    }

    private func ceilToMinute(_ date: Date) -> Date {
        let floor = floorToMinute(date)
        guard floor < date else { return floor }
        return pickerCalendar.date(byAdding: .minute, value: 1, to: floor) ?? floor
    }

    private var rangeSummary: String {
        "\(date(for: currentStartTimestamp).formatted(dateSummaryStyle)) – \(date(for: currentEndTimestamp).formatted(dateSummaryStyle))"
    }

    private func date(for milliseconds: Int64) -> Date {
        Date(timeIntervalSince1970: TimeInterval(milliseconds) / 1_000)
    }

    private func timestamp(for date: Date) -> Int64 {
        Int64(date.timeIntervalSince1970 * 1_000)
    }
}

private struct NexaDatePickerAppearance: ViewModifier {
    let backgroundColor: Color?
    let pickerBackgroundColor: Color?
    let borderColor: Color?
    let borderWidth: CGFloat?
    let cornerRadius: CGFloat?

    func body(content: Content) -> some View {
        let radius = cornerRadius ?? 12
        content
            .padding(pickerBackgroundColor == nil ? 0 : 4)
            .background(pickerBackgroundColor)
            .background(backgroundColor)
            .clipShape(RoundedRectangle(cornerRadius: radius, style: .continuous))
            .overlay {
                if let borderColor, let borderWidth {
                    RoundedRectangle(cornerRadius: radius, style: .continuous)
                        .stroke(borderColor, lineWidth: borderWidth)
                }
            }
    }
}
