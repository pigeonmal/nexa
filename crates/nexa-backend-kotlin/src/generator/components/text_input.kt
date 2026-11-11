// nexa-codegen: NexaTextInputPrimitive
@androidx.compose.runtime.Composable
internal fun NexaTextInputPrimitive(
    value: String,
    onValueChange: (String) -> Unit,
    initialFocus: Boolean = false,
    componentRef: NexaTextInputRef? = null,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    focusModifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    placeholder: String = "",
    searchable: Boolean = false,
    clearable: Boolean = false,
    searchIcon: androidx.compose.ui.graphics.vector.ImageVector? = null,
    textStyle: androidx.compose.ui.text.TextStyle = androidx.compose.material3.LocalTextStyle.current.copy(fontSize = 17.sp),
    textColor: androidx.compose.ui.graphics.Color? = null,
    placeholderColor: androidx.compose.ui.graphics.Color? = null,
    maxLength: Int = Int.MAX_VALUE,
    minLines: Int = 1,
    maxLines: Int = Int.MAX_VALUE,
    singleLine: Boolean = true,
    secure: Boolean = false,
    keyboardOptions: androidx.compose.foundation.text.KeyboardOptions = androidx.compose.foundation.text.KeyboardOptions(),
    keyboardActions: androidx.compose.foundation.text.KeyboardActions = androidx.compose.foundation.text.KeyboardActions(),
) {
    val nativeKeyboardOptions = if (secure) keyboardOptions.copy(
        keyboardType = when (keyboardOptions.keyboardType) {
            androidx.compose.ui.text.input.KeyboardType.Text -> androidx.compose.ui.text.input.KeyboardType.Password
            androidx.compose.ui.text.input.KeyboardType.Number -> androidx.compose.ui.text.input.KeyboardType.NumberPassword
            else -> keyboardOptions.keyboardType
        },
        autoCorrectEnabled = false,
        capitalization = androidx.compose.ui.text.input.KeyboardCapitalization.None,
    ) else keyboardOptions
    val limiter = androidx.compose.runtime.remember { NexaTextInputEditLimiter() }
    val initialFocusOnMount = androidx.compose.runtime.remember { initialFocus }
    val inputHasFocus = androidx.compose.runtime.remember {
        androidx.compose.runtime.mutableStateOf(false)
    }
    val focusManager = androidx.compose.ui.platform.LocalFocusManager.current
    val focusRequester = if (initialFocusOnMount || componentRef != null) {
        androidx.compose.runtime.remember { androidx.compose.ui.focus.FocusRequester() }
    } else {
        null
    }
    androidx.compose.runtime.LaunchedEffect(focusRequester) {
        if (initialFocusOnMount) {
            androidx.compose.runtime.withFrameNanos { }
            focusRequester?.requestFocus()
        }
    }
    if (componentRef != null && focusRequester != null) {
        androidx.compose.runtime.DisposableEffect(componentRef, focusRequester, focusManager) {
            val generation = componentRef.attach(
                focus = { focusRequester.requestFocus() },
                blur = {
                    if (inputHasFocus.value) focusManager.clearFocus(force = true)
                },
            )
            onDispose { componentRef.detach(generation) }
        }
    }
    val acceptEdit: (String) -> Unit = { proposed ->
        val accepted = limiter.edit(value, proposed, maxLength, maxLines, !singleLine)
        if (accepted != value) onValueChange(accepted)
    }
    val bringIntoViewRequester = androidx.compose.runtime.remember {
        androidx.compose.foundation.relocation.BringIntoViewRequester()
    }
    val keyboardInsets = androidx.compose.foundation.layout.WindowInsets.ime
    val density = androidx.compose.ui.platform.LocalDensity.current
    val inputFocusModifier = focusModifier
        .then(focusRequester?.let { androidx.compose.ui.Modifier.focusRequester(it) } ?: androidx.compose.ui.Modifier)
        .bringIntoViewRequester(bringIntoViewRequester)
        .onFocusChanged { inputHasFocus.value = it.isFocused }
    androidx.compose.runtime.LaunchedEffect(inputHasFocus.value, keyboardInsets, density) {
        if (inputHasFocus.value) {
            // Observe visibility, not animated inset pixels. Hidden/unfocused
            // fields do no keyboard-animation work and no per-frame relayout.
            androidx.compose.runtime.snapshotFlow { keyboardInsets.getBottom(density) > 0 }
                .collect { visible -> if (visible) bringIntoViewRequester.bringIntoView() }
        }
    }
    if (searchable) {
        androidx.compose.foundation.layout.Row(
            modifier = modifier,
            horizontalArrangement = if (searchIcon != null) {
                androidx.compose.foundation.layout.Arrangement.spacedBy(androidx.compose.ui.unit.Dp(8f))
            } else {
                androidx.compose.foundation.layout.Arrangement.Start
            },
            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
        ) {
            if (searchIcon != null) {
                androidx.compose.material3.Icon(
                    imageVector = searchIcon,
                    contentDescription = null,
                    modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(20f)),
                    tint = androidx.compose.ui.graphics.Color(0xFF8E8E93),
                )
            }
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier.weight(1f),
                contentAlignment = androidx.compose.ui.Alignment.CenterStart,
            ) {
                NexaTextInputField(
                    value = value,
                    onValueChange = acceptEdit,
                    modifier = inputFocusModifier.fillMaxWidth(),
                    placeholder = placeholder,
                    textStyle = textStyle,
                    textColor = textColor,
                    placeholderColor = placeholderColor,
                    minLines = minLines,
                    maxLines = maxLines,
                    singleLine = singleLine,
                    secure = secure,
                    keyboardOptions = nativeKeyboardOptions,
                    keyboardActions = keyboardActions,
                )
            }
            if (clearable) {
                NexaTextInputClearButtonSlot(visible = value.isNotEmpty()) {
                    acceptEdit("")
                }
            }
        }
    } else if (clearable) {
        androidx.compose.foundation.layout.Row(
            modifier = modifier,
            horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(
                androidx.compose.ui.unit.Dp(8f),
            ),
            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
        ) {
            NexaTextInputField(
                value = value,
                onValueChange = acceptEdit,
                modifier = androidx.compose.ui.Modifier.weight(1f).then(inputFocusModifier),
                placeholder = placeholder,
                textStyle = textStyle,
                textColor = textColor,
                placeholderColor = placeholderColor,
                minLines = minLines,
                maxLines = maxLines,
                singleLine = singleLine,
                secure = secure,
                keyboardOptions = nativeKeyboardOptions,
                keyboardActions = keyboardActions,
            )
            NexaTextInputClearButtonSlot(visible = value.isNotEmpty()) {
                acceptEdit("")
            }
        }
    } else {
        NexaTextInputField(
            value = value,
            onValueChange = acceptEdit,
            modifier = modifier.then(inputFocusModifier),
            placeholder = placeholder,
            textStyle = textStyle,
            textColor = textColor,
            placeholderColor = placeholderColor,
            minLines = minLines,
            maxLines = maxLines,
            singleLine = singleLine,
            secure = secure,
            keyboardOptions = nativeKeyboardOptions,
            keyboardActions = keyboardActions,
        )
    }
}

@androidx.compose.runtime.Composable
private fun NexaTextInputField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: androidx.compose.ui.Modifier,
    placeholder: String,
    textStyle: androidx.compose.ui.text.TextStyle,
    textColor: androidx.compose.ui.graphics.Color?,
    placeholderColor: androidx.compose.ui.graphics.Color?,
    minLines: Int,
    maxLines: Int,
    singleLine: Boolean,
    secure: Boolean,
    keyboardOptions: androidx.compose.foundation.text.KeyboardOptions,
    keyboardActions: androidx.compose.foundation.text.KeyboardActions,
) {
    val inputStyle = textStyle.copy(
        color = textColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
        textAlign = androidx.compose.ui.text.style.TextAlign.Start,
        fontWeight = androidx.compose.ui.text.font.FontWeight.Normal,
        letterSpacing = 0.sp,
        lineHeight = androidx.compose.ui.unit.TextUnit.Unspecified,
    )
    androidx.compose.foundation.text.BasicTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier,
        textStyle = inputStyle,
        minLines = minLines,
        maxLines = maxLines,
        singleLine = singleLine,
        visualTransformation = if (secure) {
            androidx.compose.ui.text.input.PasswordVisualTransformation()
        } else {
            androidx.compose.ui.text.input.VisualTransformation.None
        },
        keyboardOptions = keyboardOptions,
        keyboardActions = keyboardActions,
        cursorBrush = androidx.compose.ui.graphics.SolidColor(
            androidx.compose.material3.MaterialTheme.colorScheme.primary,
        ),
        decorationBox = { innerTextField ->
            androidx.compose.foundation.layout.Box(
                contentAlignment = if (singleLine) androidx.compose.ui.Alignment.CenterStart else androidx.compose.ui.Alignment.TopStart,
            ) {
                if (value.isEmpty() && placeholder.isNotEmpty()) {
                    androidx.compose.material3.Text(
                        placeholder,
                        style = inputStyle.copy(
                            color = placeholderColor ?: androidx.compose.ui.graphics.Color(0xFF8E8E93),
                        ),
                        maxLines = 1,
                    )
                }
                innerTextField()
            }
        },
    )
}

@androidx.compose.runtime.Composable
private fun NexaTextInputClearButton(onClear: () -> Unit) {
    androidx.compose.material3.IconButton(onClick = onClear) {
        androidx.compose.material3.Icon(
            imageVector = androidx.compose.material.icons.Icons.Filled.Cancel,
            contentDescription = "Clear text",
            modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(18f)),
            tint = androidx.compose.ui.graphics.Color(0xFF8E8E93),
        )
    }
}

@androidx.compose.runtime.Composable
private fun NexaTextInputClearButtonSlot(
    visible: Boolean,
    onClear: () -> Unit,
) {
    // Keep the Material touch-target slot present while empty. Otherwise
    // revealing the clear button increases the input row from text height to
    // IconButton's 48 dp minimum height as soon as the user types.
    androidx.compose.foundation.layout.Box(
        modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(48f)),
        contentAlignment = androidx.compose.ui.Alignment.Center,
    ) {
        if (visible) NexaTextInputClearButton(onClear)
    }
}

