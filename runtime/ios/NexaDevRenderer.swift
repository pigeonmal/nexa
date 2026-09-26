import Foundation
import SwiftUI
import UIKit

struct NexaDevContentSlot: @unchecked Sendable {
    let nodes: [Any]
    let scope: String
    let parameters: [String: Any]
}

struct NexaDevContentSlotKey: EnvironmentKey {
    static let defaultValue: NexaDevContentSlot? = nil
}

extension EnvironmentValues {
    var nexaDevContentSlot: NexaDevContentSlot? {
        get { self[NexaDevContentSlotKey.self] }
        set { self[NexaDevContentSlotKey.self] = newValue }
    }
}

struct NexaDevListPositionPreference: PreferenceKey {
    static let defaultValue: [Int: CGFloat] = [:]

    static func reduce(value: inout [Int: CGFloat], nextValue: () -> [Int: CGFloat]) {
        value.merge(nextValue(), uniquingKeysWith: { _, newest in newest })
    }
}

@MainActor
enum NexaDevFastListAxis {
    case vertical
    case horizontal
    case grid(Int)

    var scrollAxis: Axis.Set {
        if case .horizontal = self { return .horizontal }
        return .vertical
    }

    var anchor: UnitPoint {
        switch self {
        case .vertical: .top
        case .horizontal: .leading
        case .grid: .topLeading
        }
    }

    var tracksHorizontalOffset: Bool {
        if case .horizontal = self { return true }
        return false
    }
}

@MainActor
struct NexaDevFastList: View {
    let count: Int
    let axis: NexaDevFastListAxis
    let rowHeight: Double?
    let scrollPosition: Int?
    let sectionCounts: [Int]?
    let stickyHeader: AnyView?
    let sectionHeader: ((Int) -> AnyView)?
    let row: (Int, Int, Int) -> AnyView
    let onScrollPositionChanged: (Int) -> Void
    let onScroll: ((Int) -> Void)?
    let onEndReached: ((Int) -> Void)?
    let isRefreshing: Bool
    let onRefresh: (() -> Void)?
    @State private var canUpdateScrollPosition = false
    @State private var didReachEnd = false

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView(axis.scrollAxis) {
                listContent
            }
            .refreshable {
                onRefresh?()
            }
            .coordinateSpace(name: "nexa-dev-fast-list")
            .onAppear {
                if let scrollPosition, (0..<count).contains(scrollPosition) {
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                        proxy.scrollTo(scrollPosition, anchor: axis.anchor)
                    }
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                        canUpdateScrollPosition = true
                    }
                } else {
                    canUpdateScrollPosition = true
                }
            }
            .onChange(of: scrollPosition) { index in
                guard let index, (0..<count).contains(index) else { return }
                canUpdateScrollPosition = false
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                    proxy.scrollTo(index, anchor: axis.anchor)
                }
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                    canUpdateScrollPosition = true
                }
            }
            .onPreferenceChange(NexaDevListPositionPreference.self) { offsets in
                let visibleRows = offsets.filter { $0.value >= 0 }
                guard let visibleIndex = visibleRows
                    .min(by: { abs($0.value) < abs($1.value) })?.key
                else { return }
                onScroll?(visibleIndex)
                if canUpdateScrollPosition { onScrollPositionChanged(visibleIndex) }
                if visibleIndex >= count - 10, !didReachEnd {
                    didReachEnd = true
                    onEndReached?(visibleIndex)
                } else if visibleIndex < count - 1 {
                    didReachEnd = false
                }
            }
            .onChange(of: count) { _ in didReachEnd = false }
        }
    }

    @ViewBuilder
    private var listContent: some View {
        switch axis {
        case .vertical:
            LazyVStack(alignment: .leading, spacing: 0, pinnedViews: stickyHeader != nil || sectionCounts != nil ? [.sectionHeaders] : []) {
                if let sectionCounts {
                    ForEach(sectionCounts.indices, id: \.self) { sectionIndex in
                        let start = sectionCounts.prefix(sectionIndex).reduce(0, +)
                        Section {
                            ForEach(0..<sectionCounts[sectionIndex], id: \.self) { itemIndex in
                                listItem(start + itemIndex, section: sectionIndex, item: itemIndex, horizontal: false)
                            }
                        } header: {
                            sectionHeader?(sectionIndex) ?? AnyView(EmptyView())
                        }
                    }
                } else {
                    if let stickyHeader {
                        Section { ForEach(0..<count, id: \.self) { index in listItem(index, section: 0, item: index, horizontal: false) } }
                        header: { stickyHeader }
                    } else {
                        ForEach(0..<count, id: \.self) { index in listItem(index, section: 0, item: index, horizontal: false) }
                    }
                }
            }
        case .horizontal:
            LazyHStack(alignment: .top, spacing: 0) {
                ForEach(0..<count, id: \.self) { index in
                    listItem(index, section: 0, item: index, horizontal: true)
                }
            }
        case let .grid(columns):
            LazyVGrid(
                columns: Array(repeating: GridItem(.flexible(), spacing: 0), count: max(1, columns)),
                alignment: .leading,
                spacing: 0
            ) {
                ForEach(0..<count, id: \.self) { index in
                    listItem(index, section: 0, item: index, horizontal: false)
                }
            }
        }
    }

    private func listItem(_ index: Int, section: Int, item: Int, horizontal: Bool) -> some View {
        row(index, section, item)
            .frame(minWidth: horizontal ? rowHeight.map { CGFloat($0) } : nil)
            .frame(minHeight: horizontal ? nil : rowHeight.map { CGFloat($0) })
            .frame(maxWidth: horizontal ? nil : .infinity, alignment: .leading)
            .id(index)
            .onAppear {
                guard index >= count - 1, !didReachEnd else { return }
                didReachEnd = true
                onEndReached?(index)
            }
            .background(GeometryReader { geometry in
                let frame = geometry.frame(in: .named("nexa-dev-fast-list"))
                Color.clear.preference(
                    key: NexaDevListPositionPreference.self,
                    value: [index: axis.tracksHorizontalOffset ? frame.minX : frame.minY]
                )
            })
    }
}

@MainActor
struct NexaDevNodeList: View {
    let nodes: [Any]
    let module: [String: Any]
    @ObservedObject var store: NexaDevStateStore
    var focusedField: FocusState<String?>.Binding
    var parameters: [String: Any] = [:]
    var stateScope: String = "app"
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.nexaDevContentSlot) private var contentSlot

    var body: some View {
        let _ = store.revision
        let locals = store.locals(scope: stateScope, parameters: parameters)
        VStack(spacing: 0) {
            ForEach(nodes.indices, id: \.self) { index in
                renderNode(nodes[index], locals: locals, scope: stateScope)
            }
        }
    }

    private func renderNode(_ rawNode: Any, locals: [String: Any], scope: String) -> AnyView {
        let kind: String
        let fields: [String: Any]
        if let unitVariant = rawNode as? String {
            kind = unitVariant
            fields = [:]
        } else if let tagged = rawNode as? [String: Any], let (tag, payload) = tagged.first {
            kind = tag
            fields = payload as? [String: Any] ?? [:]
        } else {
            return AnyView(EmptyView())
        }
        switch kind {
        case "Layout":
            let children = fields["children"] as? [Any] ?? []
            let spacing = fields["spacing"] as? Double ?? 0
            let style = fields["style"] as? [String: Any] ?? [:]
            let alignment = style["alignment"] as? String
            let rowAlignment: VerticalAlignment = switch alignment {
            case "Start": .top
            case "End": .bottom
            default: .center
            }
            let stackAlignment: Alignment = switch alignment {
            case "Start": .topLeading
            case "End": .bottomTrailing
            default: .center
            }
            let columnAlignment: HorizontalAlignment = switch alignment {
            case "Start": .leading
            case "End": .trailing
            default: .center
            }
            let container: AnyView
            switch fields["kind"] as? String {
            case "Row":
                container = AnyView(HStack(alignment: rowAlignment, spacing: spacing) { ForEach(children.indices, id: \.self) { renderNode(children[$0], locals: locals, scope: scope) } })
            case "Stack":
                container = AnyView(ZStack(alignment: stackAlignment) { ForEach(children.indices, id: \.self) { renderNode(children[$0], locals: locals, scope: scope) } })
            default:
                container = AnyView(VStack(alignment: columnAlignment, spacing: spacing) { ForEach(children.indices, id: \.self) { renderNode(children[$0], locals: locals, scope: scope) } })
            }
            var styled = container
            if let padding = style["padding"] as? Double { styled = AnyView(styled.padding(padding)) }
            let width = style["width"] as? Double
            let height = style["height"] as? Double
            let minWidth = style["min_width"] as? Double
            let maxWidth = style["max_width"] as? Double
            let minHeight = style["min_height"] as? Double
            let maxHeight = style["max_height"] as? Double
            if width != nil || height != nil || minWidth != nil || maxWidth != nil || minHeight != nil || maxHeight != nil {
                styled = AnyView(styled.frame(
                    minWidth: minWidth.map { CGFloat($0) },
                    idealWidth: width.map { CGFloat($0) },
                    maxWidth: maxWidth.map { CGFloat($0) },
                    minHeight: minHeight.map { CGFloat($0) },
                    idealHeight: height.map { CGFloat($0) },
                    maxHeight: maxHeight.map { CGFloat($0) }
                ))
            }
            if let background = devColor(style["background"], isDark: colorScheme == .dark) {
                styled = AnyView(styled.background(background))
            }
            if let radius = style["corner_radius"] as? Double {
                styled = AnyView(styled.clipShape(RoundedRectangle(cornerRadius: radius)))
            }
            if let border = devColor(style["border_color"], isDark: colorScheme == .dark) {
                let width = style["border_width"] as? Double ?? 1
                styled = AnyView(styled.overlay(RoundedRectangle(cornerRadius: style["corner_radius"] as? Double ?? 0).stroke(border, lineWidth: width)))
            }
            if let opacity = style["opacity"] as? Double { styled = AnyView(styled.opacity(opacity)) }
            if let animation = style["animation"] as? String {
                let value: Animation
                switch animation {
                case "Spring": value = .spring()
                case "EaseIn": value = .easeIn
                case "EaseOut": value = .easeOut
                case "EaseInOut": value = .easeInOut
                default: value = .linear
                }
                styled = AnyView(styled.animation(value, value: store.revision))
            }
            return styled
        case "Text":
            let text = store.stringify(store.evaluate(fields["value"] ?? "", locals: locals, scope: scope))
            let style = fields["style"] as? [String: Any] ?? [:]
            var styled = AnyView(Text(text))
            if let size = style["font_size"] as? Double {
                styled = AnyView(styled.font(.system(size: size)))
            }
            let weight: Font.Weight? = switch style["font_weight"] as? String {
            case "Normal": .regular
            case "Medium": .medium
            case "Semibold": .semibold
            case "Bold": .bold
            default: nil
            }
            if let weight { styled = AnyView(styled.fontWeight(weight)) }
            if let color = devColor(style["color"], isDark: colorScheme == .dark) { styled = AnyView(styled.foregroundStyle(color)) }
            if let lineLimit = style["line_limit"] as? Int { styled = AnyView(styled.lineLimit(lineLimit)) }
            if let lineHeight = style["line_height"] as? Double { styled = AnyView(styled.lineSpacing(max(0, lineHeight - (style["font_size"] as? Double ?? lineHeight)))) }
            if let letterSpacing = style["letter_spacing"] as? Double { styled = AnyView(styled.tracking(letterSpacing)) }
            if style["selectable"] as? Bool == true { styled = AnyView(styled.textSelection(.enabled)) }
            return styled
        case "Content":
            guard let contentSlot else { return AnyView(EmptyView()) }
            return AnyView(NexaDevNodeList(
                nodes: contentSlot.nodes,
                module: module,
                store: store,
                focusedField: focusedField,
                parameters: contentSlot.parameters,
                stateScope: contentSlot.scope
            ))
        case "ComponentCall":
            let name = fields["name"] as? String ?? ""
            guard let component = (module["components"] as? [[String: Any]])?.first(where: {
                $0["name"] as? String == name
            }) else { return AnyView(EmptyView()) }
            var componentParameters: [String: Any] = [:]
            for argument in fields["arguments"] as? [[Any]] ?? [] where argument.count >= 2 {
                guard let parameter = argument[0] as? String else { continue }
                componentParameters[parameter] = store.evaluate(argument[1], locals: locals, scope: scope)
            }
            let slot = NexaDevContentSlot(
                nodes: fields["children"] as? [Any] ?? [],
                scope: scope,
                parameters: locals
            )
            return AnyView(NexaDevNodeList(
                nodes: component["body"] as? [Any] ?? [],
                module: module,
                store: store,
                focusedField: focusedField,
                parameters: componentParameters,
                stateScope: "component/\(name)"
            ).environment(\.nexaDevContentSlot, slot))
        case "Button":
            let label = store.stringify(store.evaluate(fields["label"] ?? "", locals: locals, scope: scope))
            let actions = fields["actions"] as? [Any] ?? []
            let disabled = fields["disabled"].map { store.truthy(store.evaluate($0, locals: locals, scope: scope)) } ?? false
            return AnyView(Button(label) { store.perform(actions, scope: scope, locals: locals) }.disabled(disabled))
        case "Pressable":
            let children = fields["children"] as? [Any] ?? []
            let actions = fields["actions"] as? [Any] ?? []
            let longPressActions = fields["long_press_actions"] as? [Any] ?? []
            let disabled = fields["disabled"].map { store.truthy(store.evaluate($0, locals: locals, scope: scope)) } ?? false
            let haptic = fields["haptic"] as? String
            let content = NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            let button = Button {
                playNexaHaptic(haptic)
                store.perform(actions, scope: scope, locals: locals)
            } label: {
                content
            }
            .buttonStyle(.plain)
            .disabled(disabled)
            if longPressActions.isEmpty {
                return AnyView(button)
            }
            return AnyView(button.simultaneousGesture(LongPressGesture().onEnded { _ in
                store.perform(longPressActions, scope: scope, locals: locals)
            }))
        case "TextInput":
            let name = fields["state"] as? String ?? ""
            let placeholder = fields["placeholder"] as? String ?? ""
            let identity = "\(scope)/input/\(name)"
            let maxLength = fields["max_length"] as? Int
            let binding = Binding(
                get: { store.stringify(store.value(name, scope: scope)) },
                set: { nextValue in
                    let value = maxLength.map { String(nextValue.prefix(max(0, $0))) } ?? nextValue
                    store.setValue(name, value: value, scope: scope)
                }
            )
            var input = AnyView(TextField(placeholder, text: binding, axis: fields["multiline"] as? Bool == true ? .vertical : .horizontal))
            switch fields["keyboard"] as? String {
            case "Number": input = AnyView(input.keyboardType(.numberPad))
            case "Email": input = AnyView(input.keyboardType(.emailAddress))
            case "Phone": input = AnyView(input.keyboardType(.phonePad))
            case "Url": input = AnyView(input.keyboardType(.URL))
            default: input = AnyView(input.keyboardType(.default))
            }
            let capitalization: TextInputAutocapitalization = switch fields["capitalization"] as? String {
            case "None": .never
            case "Words": .words
            case "Characters": .characters
            default: .sentences
            }
            input = AnyView(input.textInputAutocapitalization(capitalization))
            if fields["autocorrect"] as? Bool == false || fields["capitalization"] as? String == "None" {
                input = AnyView(input.autocorrectionDisabled())
            } else if fields["autocorrect"] as? Bool == true {
                input = AnyView(input.autocorrectionDisabled(false))
            }
            input = AnyView(input.focused(focusedField, equals: identity))
            if fields["secure"] as? Bool == true {
                return AnyView(SecureField(placeholder, text: binding).focused(focusedField, equals: identity))
            }
            return input
        case "FastList":
            guard let source = fields["source"] as? [String: Any] else {
                return AnyView(Text("FastList source could not be evaluated."))
            }
            let countExpression = source["Count"]
            let sourceItems: [Any]?
            if let itemSource = source["Items"] as? [String: Any],
               let itemExpression = itemSource["collection"] {
                sourceItems = store.evaluate(itemExpression, locals: locals, scope: scope) as? [Any] ?? []
            } else {
                sourceItems = nil
            }
            let sourceSections: [[Any]]? = (source["Sections"] as? [String: Any])
                .flatMap { $0["collection"] }
                .flatMap { store.evaluate($0, locals: locals, scope: scope) as? [[Any]] }
            guard countExpression != nil || sourceItems != nil || sourceSections != nil else {
                return AnyView(Text("FastList source could not be evaluated."))
            }
            let itemCount = countExpression.map {
                max(0, (store.evaluate($0, locals: locals, scope: scope) as? NSNumber)?.intValue ?? 0)
            } ?? (sourceItems?.count ?? 0)
            let count = sourceSections?.reduce(0) { $0 + $1.count } ?? itemCount
            let axis: NexaDevFastListAxis
            if fields["axis"] as? String == "Horizontal" {
                axis = .horizontal
            } else if let grid = (fields["axis"] as? [String: Any])?["Grid"] as? [String: Any],
                      let columns = grid["columns"] as? Int {
                axis = .grid(max(1, columns))
            } else {
                axis = .vertical
            }
            let indexName = fields["index"] as? String ?? "index"
            let itemName = fields["item"] as? String
            let sectionName = fields["section"] as? String ?? "section"
            let children = fields["children"] as? [Any] ?? []
            let itemExtent = fields["item_extent"] as? Double
            let scrollPositionName = fields["scroll_position"] as? String
            let requestedIndex = scrollPositionName.flatMap { name in
                (store.value(name, scope: scope) as? NSNumber)?.intValue
            }
            let stickyHeaderNodes = fields["sticky_header"] as? [Any]
            let stickyHeader = stickyHeaderNodes.map { nodes in
                AnyView(NexaDevNodeList(nodes: nodes, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
            }
            let sectionHeaderNodes = fields["section_header"] as? [Any]
            let sectionHeader: ((Int) -> AnyView)? = sectionHeaderNodes.map { nodes in
                { sectionIndex in
                    AnyView(NexaDevNodeList(
                        nodes: nodes,
                        module: module,
                        store: store,
                        focusedField: focusedField,
                        parameters: locals.merging([sectionName: sectionIndex]) { _, newest in newest },
                        stateScope: scope
                    ))
                }
            }
            let onScrollActions = fields["on_scroll"] as? [Any]
            let onEndReachedActions = fields["on_end_reached"] as? [Any]
            let refresh = fields["refresh"] as? [String: Any]
            let refreshState = refresh?["state"] as? String
            let refreshActions = refresh?["actions"] as? [Any] ?? []
            return AnyView(NexaDevFastList(
                count: count,
                axis: axis,
                rowHeight: itemExtent,
                scrollPosition: requestedIndex,
                sectionCounts: sourceSections?.map(\.count),
                stickyHeader: stickyHeader,
                sectionHeader: sectionHeader,
                row: { index, sectionIndex, itemIndex in
                    var rowLocals = locals.merging([indexName: itemIndex]) { _, newest in newest }
                    if let sourceSections, let itemName,
                       sourceSections.indices.contains(sectionIndex),
                       sourceSections[sectionIndex].indices.contains(itemIndex) {
                        rowLocals[itemName] = sourceSections[sectionIndex][itemIndex]
                        rowLocals[sectionName] = sectionIndex
                    } else if let itemName, let sourceItems, sourceItems.indices.contains(index) {
                        rowLocals[itemName] = sourceItems[index]
                    }
                    return AnyView(NexaDevNodeList(
                        nodes: children,
                        module: module,
                        store: store,
                        focusedField: focusedField,
                        parameters: rowLocals,
                        stateScope: scope
                    ))
                },
                onScrollPositionChanged: { index in
                    guard let scrollPositionName,
                          (store.value(scrollPositionName, scope: scope) as? NSNumber)?.intValue != index
                    else { return }
                    store.setValue(scrollPositionName, value: index, scope: scope)
                },
                onScroll: onScrollActions.map { actions in
                    { index in store.perform(actions, scope: scope, locals: locals.merging([indexName: index]) { _, newest in newest }) }
                },
                onEndReached: onEndReachedActions.map { actions in
                    { index in store.perform(actions, scope: scope, locals: locals.merging([indexName: index]) { _, newest in newest }) }
                },
                isRefreshing: refreshState.map { store.truthy(store.value($0, scope: scope)) } ?? false,
                onRefresh: refresh.map { _ in
                    { store.perform(refreshActions, scope: scope, locals: locals) }
                }
            ))
        case "Switch":
            let name = fields["state"] as? String ?? ""
            let label = fields["label"] as? String ?? ""
            return AnyView(Toggle(label, isOn: Binding(
                get: { store.truthy(store.value(name, scope: scope)) },
                set: { store.setValue(name, value: $0, scope: scope) }
            )))
        case "Image":
            let source = fields["source"] as? [String: Any] ?? [:]
            let description = fields["description"] as? String ?? ""
            let mode: ContentMode = fields["scale"] as? String == "Fill" ? .fill : .fit
            let placeholder = fields["placeholder"] as? String
            let rendered: AnyView
            if let asset = source["Asset"] as? String {
                rendered = AnyView(Image(asset).resizable().aspectRatio(contentMode: mode))
            } else if let expression = source["RemoteUrl"] {
                let urlText = store.stringify(store.evaluate(expression, locals: locals, scope: scope))
                rendered = AnyView(AsyncImage(url: URL(string: urlText)) { phase in
                    if let image = phase.image {
                        image.resizable().aspectRatio(contentMode: mode)
                    } else if let placeholder {
                        Image(placeholder).resizable().aspectRatio(contentMode: mode)
                    } else if phase.error != nil {
                        Image(systemName: "photo")
                    } else {
                        ProgressView()
                    }
                })
            } else {
                if let placeholder {
                    rendered = AnyView(Image(placeholder).resizable().aspectRatio(contentMode: mode))
                } else {
                    rendered = AnyView(Image(systemName: "photo"))
                }
            }
            return AnyView(rendered.accessibilityLabel(description).accessibilityHidden(description.isEmpty))
        case "RefreshControl":
            let state = fields["state"] as? String ?? ""
            let actions = fields["actions"] as? [Any] ?? []
            let children = fields["children"] as? [Any] ?? []
            return AnyView(ScrollView(.vertical) {
                NexaDevNodeList(
                    nodes: children,
                    module: module,
                    store: store,
                    focusedField: focusedField,
                    parameters: locals,
                    stateScope: scope
                )
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .refreshable {
                store.perform(actions, scope: scope, locals: locals)
                while store.truthy(store.value(state, scope: scope)) {
                    try? await Task.sleep(nanoseconds: 100_000_000)
                }
            })
        case "If":
            let condition = fields["condition"].map { store.truthy(store.evaluate($0, locals: locals, scope: scope)) } ?? false
            let children = condition
                ? fields["then_body"] as? [Any] ?? []
                : fields["else_body"] as? [Any] ?? []
            return AnyView(NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
        case "When":
            let value = store.stringify(store.evaluate(fields["value"] ?? NSNull(), locals: locals, scope: scope))
            let cases = fields["cases"] as? [[String: Any]] ?? []
            let matchingCase = cases.first { item in
                store.stringify(store.evaluate(item["value"] ?? NSNull(), locals: locals, scope: scope)) == value
            }
            let children = matchingCase?["body"] as? [Any] ?? fields["else_body"] as? [Any] ?? []
            return AnyView(NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
        case "Link":
            let urlText = store.stringify(store.evaluate(fields["url"] ?? "", locals: locals, scope: scope))
            guard let destination = URL(string: urlText) else {
                return AnyView(NexaDevNodeList(nodes: fields["children"] as? [Any] ?? [], module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
            }
            let children = fields["children"] as? [Any] ?? []
            return AnyView(Link(destination: destination) {
                NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            })
        case "Accessibility":
            let label = store.stringify(store.evaluate(fields["label"] ?? "", locals: locals, scope: scope))
            let hint = fields["hint"].flatMap { $0 is NSNull ? nil : store.stringify(store.evaluate($0, locals: locals, scope: scope)) }
            let children = fields["children"] as? [Any] ?? []
            let content = NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            let role = fields["role"] as? String
            switch role {
            case "Button": return AnyView(content.accessibilityElement(children: .combine).accessibilityLabel(label).accessibilityHint(hint ?? "").accessibilityAddTraits(.isButton))
            case "Link": return AnyView(content.accessibilityElement(children: .combine).accessibilityLabel(label).accessibilityHint(hint ?? "").accessibilityAddTraits(.isLink))
            case "Header": return AnyView(content.accessibilityElement(children: .combine).accessibilityLabel(label).accessibilityHint(hint ?? "").accessibilityAddTraits(.isHeader))
            case "Image": return AnyView(content.accessibilityElement(children: .combine).accessibilityLabel(label).accessibilityHint(hint ?? "").accessibilityAddTraits(.isImage))
            default: return AnyView(content.accessibilityElement(children: .combine).accessibilityLabel(label).accessibilityHint(hint ?? ""))
            }
        case "KeyboardAware":
            let children = fields["children"] as? [Any] ?? []
            let dismissMode = fields["dismiss"] as? String == "Interactive" ? ScrollDismissesKeyboardMode.interactively : .never
            return AnyView(ScrollView(.vertical) {
                NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            }.scrollDismissesKeyboard(dismissMode))
        case "AppBottomBar":
            let state = fields["state"] as? String ?? ""
            let tabs = fields["tabs"] as? [[String: Any]] ?? []
            return AnyView(TabView(selection: Binding(
                get: { (store.value(state, scope: scope) as? NSNumber)?.intValue ?? 0 },
                set: { store.setValue(state, value: $0, scope: scope) }
            )) {
                ForEach(tabs.indices, id: \.self) { index in
                    let tab = tabs[index]
                    NexaDevNodeList(
                        nodes: tab["children"] as? [Any] ?? [],
                        module: module,
                        store: store,
                        focusedField: focusedField,
                        parameters: locals,
                        stateScope: scope
                    )
                    .tabItem {
                        if let icon = tab["icon"] as? String, !icon.isEmpty {
                            Image(systemName: icon)
                        }
                        Text(tab["label"] as? String ?? "")
                    }
                    .tag(tab["index"] as? Int ?? index)
                    .badge(tab["badge"] as? String ?? "")
                }
            })
        case "BottomSheet":
            return AnyView(EmptyView())
        case "NavigationStack":
            return navigationStack(fields, locals: locals, scope: scope)
        case "NavigationLink":
            let screenIndex = fields["destination"] as? Int ?? -1
            let screens = module["screens"] as? [[String: Any]] ?? []
            guard screens.indices.contains(screenIndex),
                  let screenName = screens[screenIndex]["name"] as? String
            else { return AnyView(EmptyView()) }
            let children = fields["children"] as? [Any] ?? []
            let guardExpression = fields["guard"]
            let enabled = guardExpression.map { expression in
                expression is NSNull || store.truthy(store.evaluate(expression, locals: locals, scope: scope))
            } ?? true
            guard enabled else {
                return AnyView(NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
            }
            let arguments = fields["arguments"] as? [Any] ?? []
            let parameters = screens[screenIndex]["parameters"] as? [[String: Any]] ?? []
            let argumentValues: [String: Any] = Dictionary(uniqueKeysWithValues: zip(parameters, arguments).compactMap { parameter, expression -> (String, Any)? in
                guard let name = parameter["name"] as? String else { return nil }
                return (name, store.evaluate(expression, locals: locals, scope: scope))
            })
            let signature = NexaDevStateStore.canonicalSignature(parameters)
            let route = store.navigationRoute(screen: screenName, values: argumentValues, signature: signature)
            return AnyView(NavigationLink(value: route) {
                NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            })
        case "NavigationBack":
            let label = store.stringify(store.evaluate(fields["label"] ?? "Back", locals: locals, scope: scope))
            return AnyView(Button(label) {
                if !store.navigationPath.isEmpty { store.navigationPath.removeLast() }
            })
        default:
            return AnyView(EmptyView())
        }
    }

    private func navigationStack(
        _ fields: [String: Any],
        locals: [String: Any],
        scope: String
    ) -> AnyView {
        let screens = module["screens"] as? [[String: Any]] ?? []
        let rootIndex = fields["root"] as? Int ?? -1
        guard screens.indices.contains(rootIndex) else { return AnyView(EmptyView()) }
        let rootScreen = screens[rootIndex]
        let rootArguments = fields["arguments"] as? [Any] ?? []
        let rootValues = argumentValues(rootScreen, expressions: rootArguments, locals: locals, scope: scope)
        let content = renderScreen(rootScreen, parameters: rootValues)
        return AnyView(NavigationStack(path: $store.navigationPath) {
            content.navigationDestination(for: NexaDevRoute.self) { route in
                guard let destination = store.routeDestination(route),
                      let screen = screens.first(where: { $0["name"] as? String == destination.screen })
                else { return AnyView(EmptyView()) }
                return renderScreen(screen, parameters: destination.values)
            }
        })
    }

    private func argumentValues(
        _ screen: [String: Any],
        expressions: [Any],
        locals: [String: Any],
        scope: String
    ) -> [String: Any] {
        let parameters = screen["parameters"] as? [[String: Any]] ?? []
        return Dictionary(uniqueKeysWithValues: zip(parameters, expressions).compactMap { parameter, expression in
            guard let name = parameter["name"] as? String else { return nil }
            return (name, store.evaluate(expression, locals: locals, scope: scope))
        })
    }

    private func renderScreen(_ screen: [String: Any], parameters: [String: Any]) -> AnyView {
        let name = screen["name"] as? String ?? ""
        let scope = "screen/\(name)"
        return AnyView(NexaDevNodeList(
            nodes: screen["body"] as? [Any] ?? [],
            module: module,
            store: store,
            focusedField: focusedField,
            parameters: parameters,
            stateScope: scope
        )
        .onAppear {
            let actions = screen["on_appear"] as? [Any] ?? []
            if screen["on_appear_async"] as? Bool == true {
                Task { @MainActor in
                    try? await store.performAsync(actions, scope: scope, locals: parameters)
                }
            } else {
                store.perform(actions, scope: scope, locals: parameters)
            }
        }
        .onDisappear {
            store.perform(screen["on_disappear"] as? [Any] ?? [], scope: scope, locals: parameters)
        })
    }
}

@MainActor
func playNexaHaptic(_ style: String?) {
    let impactStyle: UIImpactFeedbackGenerator.FeedbackStyle
    switch style {
    case "Light": impactStyle = .light
    case "Medium": impactStyle = .medium
    case "Heavy": impactStyle = .heavy
    default: return
    }
    let generator = UIImpactFeedbackGenerator(style: impactStyle)
    generator.prepare()
    generator.impactOccurred()
}
