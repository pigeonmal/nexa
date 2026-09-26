package __NEXA_PACKAGE__

/**
 * Canonical schema definitions, protocol version constants, and decoder keys
 * shared between Nexa development servers and Android dev runtimes.
 */
internal object NexaDevSchema {
    const val PROTOCOL_VERSION: Int = 3
    const val DEV_IR_FORMAT_VERSION: Int = 1
    const val TARGET_PLATFORM: String = "android"
}

/**
 * Strongly-typed string constants for dynamic Dev IR JSON keys and message types.
 */
internal object NexaDevKeys {
    // Protocol envelope keys
    const val TYPE = "type"
    const val PAYLOAD = "payload"
    const val PROTOCOL_VERSION = "protocol_version"
    const val SESSION_TOKEN = "session_token"
    const val TARGET = "target"
    const val REVISION = "revision"
    const val BASE_REVISION = "base_revision"
    const val MODULE = "module"
    const val OPERATIONS = "operations"
    const val IDENTITIES = "identities"
    const val DIAGNOSTICS = "diagnostics"
    const val ENABLED = "enabled"
    const val REASON = "reason"

    // Client message types
    const val MSG_HELLO = "hello"
    const val MSG_ACKNOWLEDGE = "acknowledge"
    const val MSG_REQUEST_FULL_MODULE = "request_full_module"
    const val MSG_DISCONNECT = "disconnect"

    // Server message types
    const val MSG_WELCOME = "welcome"
    const val MSG_FULL_MODULE = "full_module"
    const val MSG_PATCH = "patch"
    const val MSG_DIAGNOSTICS = "diagnostics"
    const val MSG_RELOAD = "reload"
    const val MSG_RESTART = "restart"
    const val MSG_PERFORMANCE_OVERLAY = "performance_overlay"

    // Patch operations
    const val OP = "op"
    const val OP_SET = "set"
    const val OP_REMOVE = "remove"
    const val OP_PATH = "path"
    const val OP_VALUE = "value"

    // Identity fields
    const val ID = "id"
    const val KIND = "kind"

    // Module fields
    const val APP_NAME = "app_name"
    const val BODY = "body"
    const val SCREENS = "screens"
    const val STATES = "states"
    const val FUNCTIONS = "functions"
    const val STRUCTS = "structs"
    const val COMPONENTS = "components"
    const val DIRECTION = "direction"
    const val STATUS_BAR = "status_bar"
    const val ON_APPEAR = "on_appear"
    const val ON_APPEAR_ASYNC = "on_appear_async"
    const val ON_DISAPPEAR = "on_disappear"
    const val ON_ACTIVE = "on_active"
    const val ON_INACTIVE = "on_inactive"
    const val ON_BACKGROUND = "on_background"

    // Screen fields
    const val NAME = "name"
    const val PARAMETERS = "parameters"

    // State declaration fields
    const val TY = "ty"
    const val INITIAL = "initial"

    // Diagnostic fields
    const val FILE = "file"
    const val LINE = "line"
    const val COLUMN = "column"
    const val MESSAGE = "message"
    const val SEVERITY = "severity"

    // Native API namespaces
    const val NS_NETWORK = "Network"
    const val NS_FILE = "File"
    const val NS_PATH = "Path"
    const val NS_PERMISSIONS = "Permissions"
}

internal object NexaDevNodeKind {
    const val LAYOUT = "Layout"
    const val COLUMN = "Column"
    const val ROW = "Row"
    const val STACK = "Stack"
    const val TEXT = "Text"
    const val BUTTON = "Button"
    const val PRESSABLE = "Pressable"
    const val TEXT_INPUT = "TextInput"
    const val FAST_LIST = "FastList"
    const val SWITCH = "Switch"
    const val IMAGE = "Image"
    const val REFRESH_CONTROL = "RefreshControl"
    const val IF = "If"
    const val WHEN = "When"
    const val LINK = "Link"
    const val ACCESSIBILITY = "Accessibility"
    const val KEYBOARD_AWARE = "KeyboardAware"
    const val APP_BOTTOM_BAR = "AppBottomBar"
    const val BOTTOM_SHEET = "BottomSheet"
    const val NAVIGATION_STACK = "NavigationStack"
    const val NAVIGATION_LINK = "NavigationLink"
    const val NAVIGATION_BACK = "NavigationBack"
    const val CONTENT = "Content"
    const val COMPONENT_CALL = "ComponentCall"
}

internal object NexaDevActionKind {
    const val ASSIGN = "Assign"
    const val EXPRESSION = "Expression"
    const val IF = "If"
    const val FOR = "For"
    const val FOR_MAP = "ForMap"
    const val WHILE = "While"
    const val COLLECTION_MUTATION = "CollectionMutation"
    const val TRY_CATCH = "TryCatch"
    const val BREAK = "Break"
    const val CONTINUE = "Continue"
    const val NATIVE_CALL = "NativeCall"
}

internal object NexaDevCollectionMutationKind {
    const val ARRAY_APPEND = "ArrayAppend"
    const val ARRAY_REMOVE_AT = "ArrayRemoveAt"
    const val SET_INSERT = "SetInsert"
    const val SET_REMOVE = "SetRemove"
    const val MAP_SET = "MapSet"
    const val MAP_REMOVE = "MapRemove"
}

internal object NexaDevExprKind {
    const val STRING = "String"
    const val BOOL = "Bool"
    const val NUMBER = "Number"
    const val ARRAY = "Array"
    const val SET = "Set"
    const val MAP = "Map"
    const val PAIR = "Pair"
    const val TRIPLE = "Triple"
    const val ENUM_VALUE = "EnumValue"
    const val STATE = "State"
    const val INTERPOLATION = "Interpolation"
    const val ADD = "Add"
    const val NOT = "Not"
    const val BINARY = "Binary"
    const val CONTAINS = "Contains"
    const val COLLECTION_TRANSFORM = "CollectionTransform"
    const val CLOSURE = "Closure"
    const val INDEX = "Index"
    const val MEMBER = "Member"
    const val RANGE = "Range"
    const val NULL = "Null"
    const val COALESCE = "Coalesce"
    const val RESULT_OK = "ResultOk"
    const val RESULT_ERR = "ResultErr"
    const val TRY = "Try"
    const val CALL = "Call"
    const val NATIVE_CALL = "NativeCall"
    const val NETWORK_FETCH = "NetworkFetch"
    const val NETWORK_DOWNLOAD = "NetworkDownload"
    const val PATH_JOIN = "PathJoin"
    const val FILE_EXISTS = "FileExists"
    const val FILE_READ_TEXT = "FileReadText"
    const val FILE_WRITE_TEXT = "FileWriteText"
    const val FILE_DELETE = "FileDelete"
    const val PERMISSION_OP = "PermissionOp"
    const val AWAIT = "Await"
    const val TRY_AWAIT = "TryAwait"
}
