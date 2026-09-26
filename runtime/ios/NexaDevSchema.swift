import Foundation

/// Canonical schema definitions, protocol version constants, and decoder keys
/// shared between Nexa development servers and native dev runtimes.
public enum NexaDevSchema {
    /// Dev protocol version exchanged during WebSocket handshake.
    public static let protocolVersion: Int = 3

    /// Format version of the Dev IR payload.
    public static let devIRFormatVersion: Int = 1

    /// Platform identifier for iOS dev runtimes.
    public static let targetPlatform = "ios"
}

/// Strongly-typed string constants for dynamic Dev IR dictionary keys and message types.
public enum NexaDevKeys {
    // Protocol envelope keys
    public static let type = "type"
    public static let payload = "payload"
    public static let protocolVersion = "protocol_version"
    public static let sessionToken = "session_token"
    public static let target = "target"
    public static let revision = "revision"
    public static let baseRevision = "base_revision"
    public static let module = "module"
    public static let operations = "operations"
    public static let identities = "identities"
    public static let diagnostics = "diagnostics"
    public static let enabled = "enabled"
    public static let reason = "reason"

    // Client message types
    public static let msgHello = "hello"
    public static let msgAcknowledge = "acknowledge"
    public static let msgRequestFullModule = "request_full_module"
    public static let msgDisconnect = "disconnect"

    // Server message types
    public static let msgWelcome = "welcome"
    public static let msgFullModule = "full_module"
    public static let msgPatch = "patch"
    public static let msgDiagnostics = "diagnostics"
    public static let msgReload = "reload"
    public static let msgRestart = "restart"
    public static let msgPerformanceOverlay = "performance_overlay"

    // Patch operations
    public static let op = "op"
    public static let opSet = "set"
    public static let opRemove = "remove"
    public static let opPath = "path"
    public static let opValue = "value"

    // Identity fields
    public static let id = "id"
    public static let kind = "kind"

    // Module fields
    public static let appName = "app_name"
    public static let body = "body"
    public static let screens = "screens"
    public static let states = "states"
    public static let functions = "functions"
    public static let structs = "structs"
    public static let components = "components"
    public static let direction = "direction"
    public static let statusBar = "status_bar"
    public static let onAppear = "on_appear"
    public static let onAppearAsync = "on_appear_async"
    public static let onDisappear = "on_disappear"
    public static let onActive = "on_active"
    public static let onInactive = "on_inactive"
    public static let onBackground = "on_background"

    // Screen fields
    public static let name = "name"
    public static let parameters = "parameters"

    // State declaration fields
    public static let ty = "ty"
    public static let initial = "initial"

    // Diagnostic fields
    public static let file = "file"
    public static let line = "line"
    public static let column = "column"
    public static let message = "message"
    public static let severity = "severity"

    // Native API namespaces
    public static let nsNetwork = "Network"
    public static let nsFile = "File"
    public static let nsPath = "Path"
    public static let nsPermissions = "Permissions"
}

/// Known node kinds in Nexa Dev IR.
public enum NexaDevNodeKind {
    public static let layout = "Layout"
    public static let column = "Column"
    public static let row = "Row"
    public static let stack = "Stack"
    public static let text = "Text"
    public static let button = "Button"
    public static let pressable = "Pressable"
    public static let textInput = "TextInput"
    public static let fastList = "FastList"
    public static let switchNode = "Switch"
    public static let image = "Image"
    public static let refreshControl = "RefreshControl"
    public static let ifNode = "If"
    public static let whenNode = "When"
    public static let link = "Link"
    public static let accessibility = "Accessibility"
    public static let keyboardAware = "KeyboardAware"
    public static let appBottomBar = "AppBottomBar"
    public static let bottomSheet = "BottomSheet"
    public static let navigationStack = "NavigationStack"
    public static let navigationLink = "NavigationLink"
    public static let navigationBack = "NavigationBack"
    public static let content = "Content"
    public static let componentCall = "ComponentCall"
}

/// Known action kinds in Nexa Dev IR.
public enum NexaDevActionKind {
    public static let assign = "Assign"
    public static let expression = "Expression"
    public static let ifAction = "If"
    public static let forLoop = "For"
    public static let forMap = "ForMap"
    public static let whileLoop = "While"
    public static let collectionMutation = "CollectionMutation"
    public static let tryCatch = "TryCatch"
    public static let breakAction = "Break"
    public static let continueAction = "Continue"
    public static let nativeCall = "NativeCall"
}

/// Known collection mutation kinds in Nexa Dev IR.
public enum NexaDevCollectionMutationKind {
    public static let arrayAppend = "ArrayAppend"
    public static let arrayRemoveAt = "ArrayRemoveAt"
    public static let setInsert = "SetInsert"
    public static let setRemove = "SetRemove"
    public static let mapSet = "MapSet"
    public static let mapRemove = "MapRemove"
}

/// Known expression kinds in Nexa Dev IR.
public enum NexaDevExprKind {
    public static let literalString = "String"
    public static let literalBool = "Bool"
    public static let literalNumber = "Number"
    public static let array = "Array"
    public static let set = "Set"
    public static let map = "Map"
    public static let pair = "Pair"
    public static let triple = "Triple"
    public static let enumValue = "EnumValue"
    public static let state = "State"
    public static let interpolation = "Interpolation"
    public static let add = "Add"
    public static let not = "Not"
    public static let binary = "Binary"
    public static let contains = "Contains"
    public static let collectionTransform = "CollectionTransform"
    public static let closure = "Closure"
    public static let index = "Index"
    public static let member = "Member"
    public static let range = "Range"
    public static let null = "Null"
    public static let coalesce = "Coalesce"
    public static let resultOk = "ResultOk"
    public static let resultErr = "ResultErr"
    public static let tryExpr = "Try"
    public static let call = "Call"
    public static let nativeCall = "NativeCall"
    public static let networkFetch = "NetworkFetch"
    public static let networkDownload = "NetworkDownload"
    public static let pathJoin = "PathJoin"
    public static let fileExists = "FileExists"
    public static let fileReadText = "FileReadText"
    public static let fileWriteText = "FileWriteText"
    public static let fileDelete = "FileDelete"
    public static let permissionOp = "PermissionOp"
    public static let awaitExpr = "Await"
    public static let tryAwait = "TryAwait"
}
