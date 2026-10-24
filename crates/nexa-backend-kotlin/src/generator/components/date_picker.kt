@androidx.compose.runtime.Composable
internal fun NexaDatePickerPrimitive(
    timestamp: Long?,
    startTimestamp: Long?,
    endTimestamp: Long?,
    pickerType: String,
    style: String,
    selection: String,
    minimum: Long?,
    maximum: Long?,
    tint: androidx.compose.ui.graphics.Color?,
    textColor: androidx.compose.ui.graphics.Color?,
    backgroundColor: androidx.compose.ui.graphics.Color?,
    pickerBackgroundColor: androidx.compose.ui.graphics.Color?,
    borderColor: androidx.compose.ui.graphics.Color?,
    borderWidth: androidx.compose.ui.unit.Dp?,
    cornerRadius: androidx.compose.ui.unit.Dp?,
    localeTag: String?,
    timeZoneId: String?,
    is24Hour: Boolean?,
    enabled: Boolean?,
    accessibilityLabel: String?,
    onTimestampChanged: ((Long) -> Unit)?,
    onRangeChanged: ((Long, Long) -> Unit)?,
) {
    val zone = androidx.compose.runtime.remember(timeZoneId) {
        nexaPickerTimeZone(timeZoneId)
    }
    val locale = androidx.compose.runtime.remember(localeTag) {
        nexaPickerLocale(localeTag)
    }
    val system = androidx.compose.material3.MaterialTheme.colorScheme
    val primary = tint ?: system.primary
    val content = textColor ?: system.onSurface
    val selectedContent = textColor ?: system.onPrimary
    val pickerSurface = pickerBackgroundColor ?: system.surface
    val isRange = selection == "Range"
    val isCompact = style == "Compact" || (style == "Automatic" && !isRange) || style == "Input"
    val isInput = style == "Input" || style == "Wheel"
    val now = androidx.compose.runtime.remember(zone) { System.currentTimeMillis() }
    val initialTimestamp = timestamp ?: now
    // initial-only: the local values own the selection when no binding was supplied.
    val localTimestamp = androidx.compose.runtime.remember { androidx.compose.runtime.mutableLongStateOf(initialTimestamp) }
    val localStartTimestamp = androidx.compose.runtime.remember { androidx.compose.runtime.mutableLongStateOf(startTimestamp ?: initialTimestamp) }
    val localEndTimestamp = androidx.compose.runtime.remember { androidx.compose.runtime.mutableLongStateOf(endTimestamp ?: initialTimestamp) }
    val rawTimestamp = timestamp ?: localTimestamp.longValue
    val currentTimestamp = nexaConstrainPickerTimestamp(
        rawTimestamp,
        pickerType,
        minimum,
        maximum,
        zone,
        rawTimestamp,
    )
    val currentStartTimestamp = nexaConstrainPickerTimestamp(
        startTimestamp ?: localStartTimestamp.longValue,
        "Date",
        minimum,
        maximum,
        zone,
        startTimestamp ?: localStartTimestamp.longValue,
    )
    val currentEndTimestamp = nexaConstrainPickerTimestamp(
        endTimestamp ?: localEndTimestamp.longValue,
        "Date",
        minimum,
        maximum,
        zone,
        endTimestamp ?: localEndTimestamp.longValue,
    )
    val effectiveIs24Hour = is24Hour ?: nexaPickerUses24HourClock(
        locale = locale,
        localeIsExplicit = !localeTag.isNullOrBlank(),
        context = androidx.compose.ui.platform.LocalContext.current,
    )
    val minimumDayMillis = minimum?.let { nexaPickerDateMillis(it, zone) }
    val maximumDayMillis = maximum?.let { nexaPickerDateMillis(it, zone) }
    val boundsAreValid = if (pickerType == "Date" || isRange) {
        minimumDayMillis == null || maximumDayMillis == null || minimumDayMillis <= maximumDayMillis
    } else {
        minimum == null || maximum == null || minimum <= maximum
    }
    val timeHasSelectableMinute = when (pickerType) {
        "Time" -> nexaTimeHasSelectableMinute(currentTimestamp, minimum, maximum, zone)
        "DateTime" -> nexaInstantRangeHasSelectableMinute(minimum, maximum, zone)
        else -> true
    }
    val enabledValue = enabled != false && boundsAreValid && timeHasSelectableMinute
    val configuration = androidx.compose.ui.platform.LocalConfiguration.current
    val pickerConfiguration = androidx.compose.runtime.remember(configuration, localeTag, locale) {
        if (localeTag.isNullOrBlank()) configuration
        else android.content.res.Configuration(configuration).apply { setLocale(locale) }
    }
    val selectableDates = androidx.compose.runtime.remember(minimum, maximum, zone, pickerType) {
        object : androidx.compose.material3.SelectableDates {
            override fun isSelectableDate(utcTimeMillis: Long): Boolean =
                (minimumDayMillis == null || utcTimeMillis >= minimumDayMillis) &&
                    (maximumDayMillis == null || utcTimeMillis <= maximumDayMillis)

            override fun isSelectableYear(year: Int): Boolean = year in 1..9999
        }
    }
    val boundedTimestamp = nexaConstrainPickerTimestamp(
        currentTimestamp,
        pickerType,
        minimum,
        maximum,
        zone,
        currentTimestamp,
    )
    val initialDate = nexaPickerDateMillis(boundedTimestamp, zone)
    val initialStartDate = nexaPickerDateMillis(currentStartTimestamp, zone)
    val initialEndDate = nexaPickerDateMillis(currentEndTimestamp, zone)
    val boundedInitialStartDate = if (boundsAreValid) {
        initialStartDate.coerceIn(minimumDayMillis ?: Long.MIN_VALUE, maximumDayMillis ?: Long.MAX_VALUE)
    } else initialStartDate
    val boundedInitialEndDate = if (boundsAreValid) {
        initialEndDate.coerceIn(minimumDayMillis ?: Long.MIN_VALUE, maximumDayMillis ?: Long.MAX_VALUE)
    } else initialEndDate
    val orderedRange = listOf(boundedInitialStartDate, boundedInitialEndDate).sorted()
    val initialDisplayMode = if (isInput) androidx.compose.material3.DisplayMode.Input else androidx.compose.material3.DisplayMode.Picker
    val singleState = androidx.compose.runtime.remember(locale, initialDisplayMode, selectableDates) {
        androidx.compose.material3.DatePickerState(
            locale = locale,
            initialSelectedDateMillis = initialDate,
            yearRange = 1..9999,
            initialDisplayMode = initialDisplayMode,
            selectableDates = selectableDates,
        )
    }
    val rangeState = androidx.compose.runtime.remember(locale, selectableDates) {
        androidx.compose.material3.DateRangePickerState(
            locale = locale,
            initialSelectedStartDateMillis = orderedRange.first(),
            initialSelectedEndDateMillis = orderedRange.last(),
            yearRange = 1..9999,
            selectableDates = selectableDates,
        )
    }
    val timeCalendar = androidx.compose.runtime.remember(currentTimestamp, zone, locale) {
        java.util.Calendar.getInstance(zone, locale).apply { timeInMillis = currentTimestamp }
    }
    val timeState = androidx.compose.runtime.key(effectiveIs24Hour) {
        androidx.compose.material3.rememberTimePickerState(
            initialHour = timeCalendar.get(java.util.Calendar.HOUR_OF_DAY),
            initialMinute = timeCalendar.get(java.util.Calendar.MINUTE),
            is24Hour = effectiveIs24Hour,
        )
    }
    val timeInputValid = androidx.compose.runtime.remember(timeState) {
        androidx.compose.runtime.mutableStateOf(true)
    }
    val showDialog = androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(false) }
    val dialogShowsTimeOnly = androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(false) }
    val draftTimestamp = androidx.compose.runtime.remember { androidx.compose.runtime.mutableLongStateOf(currentTimestamp) }
    val draftStart = androidx.compose.runtime.remember { androidx.compose.runtime.mutableLongStateOf(currentStartTimestamp) }
    val draftEnd = androidx.compose.runtime.remember { androidx.compose.runtime.mutableLongStateOf(currentEndTimestamp) }
    val latestTimestampCallback = androidx.compose.runtime.rememberUpdatedState(onTimestampChanged)
    val latestRangeCallback = androidx.compose.runtime.rememberUpdatedState(onRangeChanged)
    val updateTimestamp: (Long) -> Unit = { value ->
        latestTimestampCallback.value?.invoke(value) ?: run { localTimestamp.longValue = value }
    }
    val updateRange: (Long, Long) -> Unit = { start, end ->
        latestRangeCallback.value?.invoke(start, end) ?: run {
            localStartTimestamp.longValue = start
            localEndTimestamp.longValue = end
        }
    }
    val shape = androidx.compose.foundation.shape.RoundedCornerShape(cornerRadius ?: 28.dp)
    val pickerColors = androidx.compose.material3.DatePickerDefaults.colors(
        containerColor = pickerSurface,
        titleContentColor = content,
        headlineContentColor = content,
        weekdayContentColor = content.copy(alpha = 0.7f),
        subheadContentColor = content.copy(alpha = 0.7f),
        navigationContentColor = content,
        dayContentColor = content,
        selectedDayContentColor = selectedContent,
        selectedDayContainerColor = primary,
        todayContentColor = primary,
        todayDateBorderColor = primary,
        dayInSelectionRangeContainerColor = primary.copy(alpha = 0.16f),
    )
    val timePickerColors = androidx.compose.material3.TimePickerDefaults.colors(
        clockDialColor = pickerSurface,
        selectorColor = primary,
        containerColor = pickerSurface,
        periodSelectorBorderColor = primary,
        periodSelectorSelectedContainerColor = primary,
        periodSelectorSelectedContentColor = selectedContent,
        timeSelectorSelectedContainerColor = primary,
        timeSelectorSelectedContentColor = selectedContent,
    )
    // Material 3 date cells are 40 dp wide, with 12 dp calendar gutters on
    // either side. A Form row can be narrower than that 304 dp minimum; keep
    // the native calendar intact and center it in the row instead of clipping
    // the first and last day columns at the Surface boundary.
    val inlineCalendarMinimumWidth = if (!isCompact && pickerType != "Time") 304.dp else 0.dp
    val outerModifier = androidx.compose.ui.Modifier
        .fillMaxWidth()
        .then(
            if (inlineCalendarMinimumWidth > 0.dp) {
                androidx.compose.ui.Modifier.requiredWidthIn(min = inlineCalendarMinimumWidth)
            } else {
                androidx.compose.ui.Modifier
            },
        )
        .clip(shape)
        .then(if (accessibilityLabel != null) androidx.compose.ui.Modifier.semantics { contentDescription = accessibilityLabel } else androidx.compose.ui.Modifier)

    androidx.compose.runtime.LaunchedEffect(currentTimestamp, zone, locale, pickerType) {
        val boundedValue = nexaConstrainPickerTimestamp(
            currentTimestamp,
            pickerType,
            minimum,
            maximum,
            zone,
            currentTimestamp,
        )
        val selectedDate = nexaPickerDateMillis(boundedValue, zone)
        if (singleState.selectedDateMillis != selectedDate) singleState.selectedDateMillis = selectedDate
        val calendar = java.util.Calendar.getInstance(zone, locale).apply { timeInMillis = boundedValue }
        if (timeState.hour != calendar.get(java.util.Calendar.HOUR_OF_DAY)) timeState.hour = calendar.get(java.util.Calendar.HOUR_OF_DAY)
        if (timeState.minute != calendar.get(java.util.Calendar.MINUTE)) timeState.minute = calendar.get(java.util.Calendar.MINUTE)
    }
    androidx.compose.runtime.LaunchedEffect(currentStartTimestamp, currentEndTimestamp, zone, isRange) {
        if (!isRange) return@LaunchedEffect
        val rawStart = nexaPickerDateMillis(currentStartTimestamp, zone)
        val rawEnd = nexaPickerDateMillis(currentEndTimestamp, zone)
        val startDate = if (boundsAreValid) rawStart.coerceIn(minimumDayMillis ?: Long.MIN_VALUE, maximumDayMillis ?: Long.MAX_VALUE) else rawStart
        val endDate = if (boundsAreValid) rawEnd.coerceIn(minimumDayMillis ?: Long.MIN_VALUE, maximumDayMillis ?: Long.MAX_VALUE) else rawEnd
        rangeState.setSelection(minOf(startDate, endDate), maxOf(startDate, endDate))
    }
    androidx.compose.runtime.LaunchedEffect(
        singleState.selectedDateMillis,
        timeState.hour,
        timeState.minute,
        currentTimestamp,
        pickerType,
        isCompact,
        isRange,
    ) {
        if (isCompact || isRange) return@LaunchedEffect
        val selectedDate = singleState.selectedDateMillis ?: return@LaunchedEffect
        val value = when (pickerType) {
            "Time" -> nexaTimestampForPickerTime(currentTimestamp, timeState.hour, timeState.minute, zone, locale)
            "DateTime" -> nexaTimestampForPickerDateTime(selectedDate, timeState.hour, timeState.minute, zone)
            else -> nexaTimestampForPickerDate(selectedDate, zone)
        }
        updateTimestamp(nexaConstrainPickerTimestamp(value, pickerType, minimum, maximum, zone, currentTimestamp))
    }
    androidx.compose.runtime.LaunchedEffect(
        rangeState.selectedStartDateMillis,
        rangeState.selectedEndDateMillis,
        isCompact,
        isRange,
    ) {
        if (isCompact || !isRange) return@LaunchedEffect
        val start = rangeState.selectedStartDateMillis ?: return@LaunchedEffect
        val end = rangeState.selectedEndDateMillis ?: return@LaunchedEffect
        val orderedStart = minOf(start, end)
        val orderedEnd = maxOf(start, end)
        updateRange(
            nexaTimestampForPickerDate(orderedStart, zone),
            nexaTimestampForPickerDate(orderedEnd, zone),
        )
    }

    androidx.compose.runtime.CompositionLocalProvider(
        androidx.compose.ui.platform.LocalConfiguration provides pickerConfiguration,
    ) {
        androidx.compose.material3.Surface(
            modifier = outerModifier,
            color = backgroundColor ?: androidx.compose.ui.graphics.Color.Transparent,
            shape = shape,
            border = if (borderColor != null && borderWidth != null) androidx.compose.foundation.BorderStroke(borderWidth, borderColor) else null,
        ) {
            androidx.compose.foundation.layout.Box(modifier = androidx.compose.ui.Modifier.fillMaxWidth()) {
                androidx.compose.foundation.layout.Column(
                    modifier = androidx.compose.ui.Modifier.fillMaxWidth().padding(if (backgroundColor != null || borderColor != null) 8.dp else 0.dp),
                    horizontalAlignment = androidx.compose.ui.Alignment.Start,
                ) {
                    if (isCompact) {
                        val isDateTime = !isRange && pickerType == "DateTime"
                        val currentText = if (isRange) {
                            nexaFormatDateRange(currentStartTimestamp, currentEndTimestamp, locale, zone)
                        } else if (isDateTime) {
                            null
                        } else {
                            nexaFormatPickerValue(currentTimestamp, pickerType, locale, zone, effectiveIs24Hour)
                        }
                        val dateTimeDateText = if (isDateTime) {
                            nexaFormatPickerValue(currentTimestamp, "Date", locale, zone, null)
                        } else null
                        val dateTimeTimeText = if (isDateTime) {
                            nexaFormatPickerValue(currentTimestamp, "Time", locale, zone, effectiveIs24Hour)
                        } else null
                        val openPicker: (Boolean) -> Unit = { showTimeOnly ->
                            draftTimestamp.longValue = currentTimestamp
                            draftStart.longValue = currentStartTimestamp
                            draftEnd.longValue = currentEndTimestamp
                            val boundedValue = nexaConstrainPickerTimestamp(
                                currentTimestamp,
                                pickerType,
                                minimum,
                                maximum,
                                zone,
                                currentTimestamp,
                            )
                            singleState.selectedDateMillis = nexaPickerDateMillis(boundedValue, zone)
                            val calendar = java.util.Calendar.getInstance(zone, locale).apply { timeInMillis = boundedValue }
                            timeState.hour = calendar.get(java.util.Calendar.HOUR_OF_DAY)
                            timeState.minute = calendar.get(java.util.Calendar.MINUTE)
                            val rangeStart = nexaPickerDateMillis(currentStartTimestamp, zone)
                                .let { if (boundsAreValid) it.coerceIn(minimumDayMillis ?: Long.MIN_VALUE, maximumDayMillis ?: Long.MAX_VALUE) else it }
                            val rangeEnd = nexaPickerDateMillis(currentEndTimestamp, zone)
                                .let { if (boundsAreValid) it.coerceIn(minimumDayMillis ?: Long.MIN_VALUE, maximumDayMillis ?: Long.MAX_VALUE) else it }
                            rangeState.setSelection(if (isRange) rangeStart else null, if (isRange) rangeEnd else null)
                            // DateTime compact controls open date and time independently.
                            dialogShowsTimeOnly.value = pickerType == "Time" || showTimeOnly
                            showDialog.value = true
                        }
                        if (dateTimeDateText != null && dateTimeTimeText != null) {
                            androidx.compose.foundation.layout.Row(
                                modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp),
                                horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(8.dp),
                                verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                            ) {
                                NexaDatePickerCompactTrigger(
                                    text = dateTimeDateText,
                                    enabled = enabledValue,
                                    contentColor = content,
                                    onClick = { openPicker(false) },
                                    modifier = Modifier.weight(1f, fill = false),
                                )
                                NexaDatePickerCompactTrigger(
                                    text = dateTimeTimeText,
                                    enabled = enabledValue,
                                    contentColor = content,
                                    onClick = { openPicker(true) },
                                    modifier = Modifier.weight(1f, fill = false),
                                )
                            }
                        } else if (currentText != null) {
                            NexaDatePickerCompactTrigger(
                                text = currentText,
                                enabled = enabledValue,
                                contentColor = content,
                                onClick = { openPicker(false) },
                                outlined = isRange,
                            )
                        }
                    } else {
                        NexaDatePickerInlineContent(
                            pickerType = pickerType,
                            style = style,
                            isRange = isRange,
                            singleState = singleState,
                            rangeState = rangeState,
                            minimumDayMillis = minimumDayMillis,
                            maximumDayMillis = maximumDayMillis,
                            initialRangeDateMillis = rangeState.selectedStartDateMillis ?: initialStartDate,
                            timeState = timeState,
                            onTimeInputValidityChanged = { timeInputValid.value = it },
                            zone = zone,
                            locale = locale,
                            colors = pickerColors,
                            timeColors = timePickerColors,
                            pickerSurface = pickerSurface,
                            textColor = content,
                            accentColor = primary,
                            selectedContentColor = selectedContent,
                        )
                    }
                }
                if (!enabledValue) {
                    androidx.compose.foundation.layout.Box(
                        modifier = androidx.compose.ui.Modifier
                            .matchParentSize()
                            .pointerInput(enabledValue) {
                                awaitPointerEventScope {
                                    while (true) {
                                        awaitPointerEvent(androidx.compose.ui.input.pointer.PointerEventPass.Initial)
                                            .changes.forEach { it.consume() }
                                    }
                                }
                            }
                            .clearAndSetSemantics {
                                accessibilityLabel?.let { contentDescription = it }
                                disabled()
                            },
                    )
                }
            }
        }

    if (showDialog.value) {
        val screenHeightDp = androidx.compose.ui.platform.LocalConfiguration.current.screenHeightDp
        val screenHeight = screenHeightDp.dp
        val dialogMaxHeight = screenHeight * 0.88f
        val calendarMaxHeight = (dialogMaxHeight - 72.dp).coerceAtLeast(0.dp)
        androidx.compose.ui.window.Dialog(
            onDismissRequest = { showDialog.value = false },
            properties = androidx.compose.ui.window.DialogProperties(
                usePlatformDefaultWidth = false,
                decorFitsSystemWindows = false,
            ),
        ) {
            val dialogView = androidx.compose.ui.platform.LocalView.current
            val dialogWindow = androidx.compose.runtime.remember(dialogView) {
                nexaDatePickerDialogWindow(dialogView)
            }
            androidx.compose.runtime.DisposableEffect(dialogWindow) {
                dialogWindow?.setWindowAnimations(0)
                onDispose {}
            }
            androidx.compose.material3.Surface(
                modifier = androidx.compose.ui.Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 8.dp)
                    .widthIn(max = 640.dp)
                    .heightIn(max = dialogMaxHeight),
                shape = cornerRadius?.let { androidx.compose.foundation.shape.RoundedCornerShape(it) }
                    ?: androidx.compose.material3.DatePickerDefaults.shape,
                color = pickerSurface,
                border = if (borderColor != null && borderWidth != null) androidx.compose.foundation.BorderStroke(borderWidth, borderColor) else null,
                tonalElevation = 6.dp,
            ) {
                androidx.compose.foundation.layout.Column(
                    modifier = androidx.compose.ui.Modifier.fillMaxWidth(),
                    horizontalAlignment = androidx.compose.ui.Alignment.CenterHorizontally,
                ) {
                    if (dialogShowsTimeOnly.value) {
                        androidx.compose.foundation.layout.Box(
                            modifier = androidx.compose.ui.Modifier.fillMaxWidth().padding(top = 24.dp),
                            contentAlignment = androidx.compose.ui.Alignment.Center,
                        ) {
                            NexaDatePickerTimeInput(timeState, style, timePickerColors, pickerSurface, content, primary, selectedContent) {
                                timeInputValid.value = it
                            }
                        }
                    } else if (isRange) {
                        NexaDatePickerSingleMonthRangeCalendar(
                            state = rangeState,
                            initialDateMillis = rangeState.selectedStartDateMillis ?: initialStartDate,
                            minimumDateMillis = minimumDayMillis,
                            maximumDateMillis = maximumDayMillis,
                            zone = zone,
                            locale = locale,
                            textColor = content,
                            accentColor = primary,
                            selectedContentColor = selectedContent,
                            showTitle = true,
                            modifier = Modifier.fillMaxWidth().heightIn(max = calendarMaxHeight),
                        )
                    } else {
                        NexaDatePickerCalendarContainer(
                            modifier = Modifier.fillMaxWidth().heightIn(max = 440.dp),
                        ) { calendarModifier ->
                            androidx.compose.material3.DatePicker(
                                state = singleState,
                                modifier = calendarModifier.heightIn(max = 440.dp),
                                title = null,
                                headline = null,
                                showModeToggle = true,
                                colors = pickerColors,
                                focusRequester = null,
                            )
                        }
                    }
                    androidx.compose.foundation.layout.Row(
                        modifier = androidx.compose.ui.Modifier.fillMaxWidth().heightIn(min = 56.dp).padding(horizontal = 16.dp, vertical = 8.dp),
                        horizontalArrangement = androidx.compose.foundation.layout.Arrangement.End,
                    ) {
                        androidx.compose.material3.TextButton(
                            onClick = { showDialog.value = false },
                            colors = androidx.compose.material3.ButtonDefaults.textButtonColors(contentColor = content),
                        ) {
                            androidx.compose.material3.Text(androidx.compose.ui.res.stringResource(android.R.string.cancel))
                        }
                        androidx.compose.material3.TextButton(
                            onClick = {
                                if (pickerType == "Time") {
                                    val value = nexaTimestampForPickerTime(draftTimestamp.longValue, timeState.hour, timeState.minute, zone, locale)
                                    updateTimestamp(nexaConstrainPickerTimestamp(value, pickerType, minimum, maximum, zone, draftTimestamp.longValue))
                                } else if (isRange) {
                                    val start = rangeState.selectedStartDateMillis?.let { nexaTimestampForPickerDate(it, zone) }
                                    val end = rangeState.selectedEndDateMillis?.let { nexaTimestampForPickerDate(it, zone) }
                                    if (start != null && end != null) updateRange(minOf(start, end), maxOf(start, end))
                                } else {
                                    singleState.selectedDateMillis?.let { selected ->
                                        val value = if (pickerType == "DateTime") {
                                            nexaTimestampForPickerDateTime(selected, timeState.hour, timeState.minute, zone)
                                        } else {
                                            nexaTimestampForPickerDate(selected, zone)
                                        }
                                        updateTimestamp(nexaConstrainPickerTimestamp(value, pickerType, minimum, maximum, zone, draftTimestamp.longValue))
                                    }
                                }
                                showDialog.value = false
                            },
                            enabled = enabledValue &&
                                ((style != "Input" && style != "Wheel") || timeInputValid.value) &&
                                (!isRange || (
                                rangeState.selectedStartDateMillis != null &&
                                    rangeState.selectedEndDateMillis != null &&
                                    rangeState.selectedStartDateMillis != rangeState.selectedEndDateMillis
                                )),
                            colors = androidx.compose.material3.ButtonDefaults.textButtonColors(contentColor = primary),
                        ) {
                            androidx.compose.material3.Text(androidx.compose.ui.res.stringResource(android.R.string.ok))
                        }
                    }
                }
            }
        }
    }
    }
}

private fun nexaDatePickerDialogWindow(view: android.view.View): android.view.Window? {
    var parent: android.view.ViewParent? = view.parent
    while (parent != null) {
        if (parent is androidx.compose.ui.window.DialogWindowProvider) return parent.window
        parent = (parent as? android.view.View)?.parent
    }
    return null
}

@androidx.compose.runtime.Composable
private fun NexaDatePickerInlineContent(
    pickerType: String,
    style: String,
    isRange: Boolean,
    singleState: androidx.compose.material3.DatePickerState,
    rangeState: androidx.compose.material3.DateRangePickerState,
    minimumDayMillis: Long?,
    maximumDayMillis: Long?,
    initialRangeDateMillis: Long,
    timeState: androidx.compose.material3.TimePickerState,
    onTimeInputValidityChanged: (Boolean) -> Unit,
    zone: java.util.TimeZone,
    locale: java.util.Locale,
    colors: androidx.compose.material3.DatePickerColors,
    timeColors: androidx.compose.material3.TimePickerColors,
    pickerSurface: androidx.compose.ui.graphics.Color,
    textColor: androidx.compose.ui.graphics.Color,
    accentColor: androidx.compose.ui.graphics.Color,
    selectedContentColor: androidx.compose.ui.graphics.Color,
) {
    if (pickerType == "Time") {
        NexaDatePickerTimeInput(timeState, style, timeColors, pickerSurface, textColor, accentColor, selectedContentColor, onTimeInputValidityChanged)
    } else if (isRange) {
        NexaDatePickerSingleMonthRangeCalendar(
            state = rangeState,
            initialDateMillis = initialRangeDateMillis,
            minimumDateMillis = minimumDayMillis,
            maximumDateMillis = maximumDayMillis,
            zone = zone,
            locale = locale,
            textColor = textColor,
            accentColor = accentColor,
            selectedContentColor = selectedContentColor,
            showTitle = false,
            modifier = Modifier.fillMaxWidth().heightIn(max = 440.dp),
        )
    } else if (pickerType == "DateTime") {
        androidx.compose.foundation.layout.Column(modifier = Modifier.fillMaxWidth()) {
            NexaDatePickerCalendarContainer(Modifier.fillMaxWidth()) { calendarModifier ->
                androidx.compose.material3.DatePicker(
                    state = singleState,
                    modifier = calendarModifier,
                    title = null,
                    headline = null,
                    showModeToggle = true,
                    colors = colors,
                    focusRequester = null,
                )
            }
            NexaDatePickerTimeInput(timeState, style, timeColors, pickerSurface, textColor, accentColor, selectedContentColor, onTimeInputValidityChanged)
        }
    } else {
        NexaDatePickerCalendarContainer(Modifier.fillMaxWidth()) { calendarModifier ->
            androidx.compose.material3.DatePicker(
                state = singleState,
                modifier = calendarModifier,
                title = null,
                headline = null,
                showModeToggle = true,
                colors = colors,
                focusRequester = null,
            )
        }
    }
}

@androidx.compose.runtime.Composable
private fun NexaDatePickerCalendarContainer(
    modifier: androidx.compose.ui.Modifier,
    content: @androidx.compose.runtime.Composable (androidx.compose.ui.Modifier) -> Unit,
) {
    androidx.compose.foundation.layout.BoxWithConstraints(modifier.fillMaxWidth()) {
        val dayCellSize = ((maxWidth - 24.dp).coerceAtLeast(0.dp) / 7).coerceAtMost(48.dp)
        androidx.compose.runtime.CompositionLocalProvider(
            androidx.compose.material3.LocalMinimumInteractiveComponentSize provides dayCellSize,
        ) {
            content(Modifier.fillMaxWidth())
        }
    }
}

@androidx.compose.runtime.Composable
private fun NexaDatePickerSingleMonthRangeCalendar(
    state: androidx.compose.material3.DateRangePickerState,
    initialDateMillis: Long,
    minimumDateMillis: Long?,
    maximumDateMillis: Long?,
    zone: java.util.TimeZone,
    locale: java.util.Locale,
    textColor: androidx.compose.ui.graphics.Color,
    accentColor: androidx.compose.ui.graphics.Color,
    selectedContentColor: androidx.compose.ui.graphics.Color,
    showTitle: Boolean,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
) {
    val utc = androidx.compose.runtime.remember { java.util.TimeZone.getTimeZone("UTC") }
    val monthPattern = androidx.compose.runtime.remember(locale) {
        android.text.format.DateFormat.getBestDateTimePattern(locale, "yMMMM")
    }
    val monthFormat = androidx.compose.runtime.remember(locale, monthPattern, utc) {
        java.text.SimpleDateFormat(monthPattern, locale).apply { timeZone = utc }
    }
    val fullDateFormat = androidx.compose.runtime.remember(locale, utc) {
        java.text.SimpleDateFormat("EEEE, d MMMM y", locale).apply { timeZone = utc }
    }
    val weekdayLabels = androidx.compose.runtime.remember(locale, utc) {
        val calendar = java.util.Calendar.getInstance(utc, locale)
        val shortWeekdays = java.text.DateFormatSymbols(locale).shortWeekdays
        (0 until 7).map { offset ->
            val dayOfWeek = (calendar.firstDayOfWeek - 1 + offset) % 7 + 1
            shortWeekdays[dayOfWeek].uppercase(locale)
        }
    }
    val initialMonthMillis = androidx.compose.runtime.remember(state, locale) {
        nexaPickerMonthStartMillis(initialDateMillis, locale)
    }
    var displayedMonthMillis by androidx.compose.runtime.remember(state, locale) {
        androidx.compose.runtime.mutableLongStateOf(initialMonthMillis)
    }
    val monthCalendar = androidx.compose.runtime.remember(displayedMonthMillis, locale, utc) {
        java.util.Calendar.getInstance(utc, locale).apply {
            timeInMillis = displayedMonthMillis
            set(java.util.Calendar.DAY_OF_MONTH, 1)
        }
    }
    val monthStartMillis = monthCalendar.timeInMillis
    val month = monthCalendar.get(java.util.Calendar.MONTH)
    val year = monthCalendar.get(java.util.Calendar.YEAR)
    val dayCount = monthCalendar.getActualMaximum(java.util.Calendar.DAY_OF_MONTH)
    val leadingDays = (monthCalendar.get(java.util.Calendar.DAY_OF_WEEK) - monthCalendar.firstDayOfWeek + 7) % 7
    val weekCount = (leadingDays + dayCount + 6) / 7
    val monthLabel = monthFormat.format(java.util.Date(monthStartMillis))
    val firstSelectableMonth = minimumDateMillis?.let { nexaPickerMonthStartMillis(it, locale) }
    val lastSelectableMonth = maximumDateMillis?.let { nexaPickerMonthStartMillis(it, locale) }
    val canShowPreviousMonth = firstSelectableMonth == null || monthStartMillis > firstSelectableMonth
    val canShowNextMonth = lastSelectableMonth == null || monthStartMillis < lastSelectableMonth
    val selectedStartMillis = state.selectedStartDateMillis
    val selectedEndMillis = state.selectedEndDateMillis
    val todayMillis = nexaPickerDateMillis(System.currentTimeMillis(), zone)
    val rangeColor = accentColor.copy(alpha = 0.16f)

    androidx.compose.foundation.layout.Column(
        modifier = modifier.fillMaxWidth(),
        horizontalAlignment = androidx.compose.ui.Alignment.Start,
    ) {
        if (showTitle) {
            androidx.compose.material3.Text(
                text = androidx.compose.ui.res.stringResource(
                    id = androidx.compose.material3.R.string.m3c_date_range_picker_title,
                ),
                modifier = androidx.compose.ui.Modifier.fillMaxWidth().padding(start = 24.dp, top = 16.dp, end = 24.dp),
                style = androidx.compose.material3.MaterialTheme.typography.titleMedium,
                color = textColor,
            )
            val summary = nexaFormatDateRange(
                start = selectedStartMillis?.let { nexaTimestampForPickerDate(it, zone) },
                end = selectedEndMillis?.let { nexaTimestampForPickerDate(it, zone) },
                locale = locale,
                zone = zone,
                dateStyle = java.text.DateFormat.SHORT,
            )
            androidx.compose.material3.Text(
                text = summary,
                modifier = androidx.compose.ui.Modifier.fillMaxWidth().padding(start = 24.dp, top = 4.dp, end = 24.dp, bottom = 8.dp),
                style = androidx.compose.material3.MaterialTheme.typography.bodyLarge,
                color = textColor,
                maxLines = 1,
                softWrap = false,
                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
            )
        }
        androidx.compose.foundation.layout.Row(
            modifier = androidx.compose.ui.Modifier.fillMaxWidth().height(48.dp).padding(horizontal = 8.dp),
            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
        ) {
            androidx.compose.material3.Text(
                text = monthLabel,
                modifier = androidx.compose.ui.Modifier.weight(1f).padding(start = 8.dp),
                color = textColor,
                style = androidx.compose.material3.MaterialTheme.typography.titleMedium,
                maxLines = 1,
                softWrap = false,
                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
            )
            androidx.compose.material3.IconButton(
                onClick = { displayedMonthMillis = nexaPickerAddMonths(displayedMonthMillis, -1, locale) },
                enabled = canShowPreviousMonth,
                modifier = androidx.compose.ui.Modifier.size(40.dp),
            ) {
                androidx.compose.material3.Icon(
                    imageVector = androidx.compose.material.icons.Icons.Filled.ChevronRight,
                    contentDescription = androidx.compose.ui.res.stringResource(
                        id = androidx.compose.material3.R.string.m3c_date_picker_switch_to_previous_month,
                    ),
                    modifier = androidx.compose.ui.Modifier.rotate(180f),
                    tint = if (canShowPreviousMonth) textColor else textColor.copy(alpha = 0.38f),
                )
            }
            androidx.compose.material3.IconButton(
                onClick = { displayedMonthMillis = nexaPickerAddMonths(displayedMonthMillis, 1, locale) },
                enabled = canShowNextMonth,
                modifier = androidx.compose.ui.Modifier.size(40.dp),
            ) {
                androidx.compose.material3.Icon(
                    imageVector = androidx.compose.material.icons.Icons.Filled.ChevronRight,
                    contentDescription = androidx.compose.ui.res.stringResource(
                        id = androidx.compose.material3.R.string.m3c_date_picker_switch_to_next_month,
                    ),
                    tint = if (canShowNextMonth) textColor else textColor.copy(alpha = 0.38f),
                )
            }
        }
        androidx.compose.foundation.layout.Row(modifier = androidx.compose.ui.Modifier.fillMaxWidth()) {
            weekdayLabels.forEach { weekday ->
                androidx.compose.foundation.layout.Box(
                    modifier = androidx.compose.ui.Modifier.weight(1f).height(32.dp),
                    contentAlignment = androidx.compose.ui.Alignment.Center,
                ) {
                    androidx.compose.material3.Text(
                        text = weekday,
                        color = textColor.copy(alpha = 0.7f),
                        style = androidx.compose.material3.MaterialTheme.typography.labelMedium,
                        maxLines = 1,
                    )
                }
            }
        }
        repeat(weekCount) { week ->
            androidx.compose.foundation.layout.Row(modifier = androidx.compose.ui.Modifier.fillMaxWidth()) {
                repeat(7) { weekday ->
                    val dayNumber = week * 7 + weekday - leadingDays + 1
                    if (dayNumber in 1..dayCount) {
                        val dayMillis = java.util.Calendar.getInstance(utc, locale).apply {
                            clear()
                            set(year, month, dayNumber)
                        }.timeInMillis
                        NexaDatePickerRangeDayCell(
                            dayMillis = dayMillis,
                            dayNumber = dayNumber,
                            dateDescription = fullDateFormat.format(java.util.Date(dayMillis)),
                            todayMillis = todayMillis,
                            selectedStartMillis = selectedStartMillis,
                            selectedEndMillis = selectedEndMillis,
                            minimumDateMillis = minimumDateMillis,
                            maximumDateMillis = maximumDateMillis,
                            textColor = textColor,
                            accentColor = accentColor,
                            selectedContentColor = selectedContentColor,
                            rangeColor = rangeColor,
                            onDateSelected = { selectedMillis ->
                                when {
                                    selectedStartMillis == null || selectedEndMillis != null -> {
                                        state.setSelection(selectedMillis, null)
                                    }
                                    selectedMillis < selectedStartMillis -> {
                                        state.setSelection(selectedMillis, selectedStartMillis)
                                    }
                                    else -> state.setSelection(selectedStartMillis, selectedMillis)
                                }
                            },
                        )
                    } else {
                        androidx.compose.foundation.layout.Box(
                            modifier = androidx.compose.ui.Modifier.weight(1f).height(44.dp),
                        )
                    }
                }
            }
        }
    }
}

@androidx.compose.runtime.Composable
private fun androidx.compose.foundation.layout.RowScope.NexaDatePickerRangeDayCell(
    dayMillis: Long,
    dayNumber: Int,
    dateDescription: String,
    todayMillis: Long,
    selectedStartMillis: Long?,
    selectedEndMillis: Long?,
    minimumDateMillis: Long?,
    maximumDateMillis: Long?,
    textColor: androidx.compose.ui.graphics.Color,
    accentColor: androidx.compose.ui.graphics.Color,
    selectedContentColor: androidx.compose.ui.graphics.Color,
    rangeColor: androidx.compose.ui.graphics.Color,
    onDateSelected: (Long) -> Unit,
) {
    val isStart = selectedStartMillis == dayMillis
    val isEnd = selectedEndMillis == dayMillis
    val isSelected = isStart || isEnd
    val isInRange = selectedStartMillis != null && selectedEndMillis != null &&
        dayMillis in selectedStartMillis..selectedEndMillis
    val isToday = todayMillis == dayMillis
    val isEnabled = (minimumDateMillis == null || dayMillis >= minimumDateMillis) &&
        (maximumDateMillis == null || dayMillis <= maximumDateMillis)
    val contentColor = when {
        isSelected -> selectedContentColor
        !isEnabled -> textColor.copy(alpha = 0.38f)
        isToday -> accentColor
        else -> textColor
    }
    androidx.compose.foundation.layout.Box(
        modifier = androidx.compose.ui.Modifier
            .weight(1f)
            .height(44.dp)
            .semantics(mergeDescendants = true) {
                contentDescription = dateDescription
                selected = isSelected
                if (!isEnabled) disabled()
            }
            .clickable(enabled = isEnabled, role = androidx.compose.ui.semantics.Role.Button) {
                onDateSelected(dayMillis)
            },
        contentAlignment = androidx.compose.ui.Alignment.Center,
    ) {
        if (isInRange && !isSelected) {
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier.fillMaxWidth().height(32.dp).background(rangeColor),
            )
        } else if (isInRange && selectedStartMillis != selectedEndMillis) {
            val isFirst = isStart
            val isLast = isEnd
            val segmentShape = when {
                isFirst -> androidx.compose.foundation.shape.RoundedCornerShape(
                    topStart = 16.dp,
                    bottomStart = 16.dp,
                )
                isLast -> androidx.compose.foundation.shape.RoundedCornerShape(
                    topEnd = 16.dp,
                    bottomEnd = 16.dp,
                )
                else -> androidx.compose.foundation.shape.RoundedCornerShape(0.dp)
            }
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier
                    .fillMaxWidth(if (isFirst || isLast) 0.5f else 1f)
                    .height(32.dp)
                    .align(if (isFirst) androidx.compose.ui.Alignment.CenterEnd else androidx.compose.ui.Alignment.CenterStart)
                    .background(rangeColor, segmentShape),
            )
        }
        androidx.compose.foundation.layout.Box(
            modifier = androidx.compose.ui.Modifier
                .size(36.dp)
                .then(
                    if (isSelected) {
                        androidx.compose.ui.Modifier.background(accentColor, androidx.compose.foundation.shape.CircleShape)
                    } else {
                        androidx.compose.ui.Modifier
                    },
                )
                .then(
                    if (isToday && !isSelected) {
                        androidx.compose.ui.Modifier.border(
                            width = 1.dp,
                            color = accentColor,
                            shape = androidx.compose.foundation.shape.CircleShape,
                        )
                    } else {
                        androidx.compose.ui.Modifier
                    },
                ),
            contentAlignment = androidx.compose.ui.Alignment.Center,
        ) {
            androidx.compose.material3.Text(
                text = dayNumber.toString(),
                color = contentColor,
                style = androidx.compose.material3.MaterialTheme.typography.bodyLarge,
                maxLines = 1,
            )
        }
    }
}

private fun nexaPickerMonthStartMillis(dateMillis: Long, locale: java.util.Locale): Long =
    java.util.Calendar.getInstance(java.util.TimeZone.getTimeZone("UTC"), locale).apply {
        timeInMillis = dateMillis
        set(java.util.Calendar.DAY_OF_MONTH, 1)
    }.timeInMillis

private fun nexaPickerAddMonths(dateMillis: Long, offset: Int, locale: java.util.Locale): Long =
    java.util.Calendar.getInstance(java.util.TimeZone.getTimeZone("UTC"), locale).apply {
        timeInMillis = dateMillis
        set(java.util.Calendar.DAY_OF_MONTH, 1)
        add(java.util.Calendar.MONTH, offset)
    }.timeInMillis

@androidx.compose.runtime.Composable
private fun NexaDatePickerCompactTrigger(
    text: String,
    enabled: Boolean,
    contentColor: androidx.compose.ui.graphics.Color,
    onClick: () -> Unit,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    outlined: Boolean = false,
) {
    val surface = androidx.compose.material3.MaterialTheme.colorScheme.surface
    val isDark = androidx.compose.material3.MaterialTheme.colorScheme.background == androidx.compose.ui.graphics.Color.Black
    // Match SwiftUI's compact DatePicker system-fill capsule instead of the
    // larger, colored Material button treatment.
    val compactFill = if (isDark) {
        androidx.compose.ui.graphics.Color.White.copy(alpha = 0.10f).compositeOver(surface)
    } else {
        androidx.compose.ui.graphics.Color(0xFF767680).copy(alpha = 0.12f).compositeOver(surface)
    }
    if (outlined) {
        val label: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit = {
            androidx.compose.material3.Text(
                text,
                style = androidx.compose.material3.MaterialTheme.typography.labelLarge,
                maxLines = 1,
                softWrap = false,
                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
            )
        }
        androidx.compose.material3.OutlinedButton(
            modifier = modifier,
            onClick = onClick,
            enabled = enabled,
            shape = androidx.compose.foundation.shape.CircleShape,
            colors = androidx.compose.material3.ButtonDefaults.outlinedButtonColors(contentColor = contentColor),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 16.dp, vertical = 6.dp),
            content = label,
        )
    } else {
        androidx.compose.material3.Surface(
            onClick = onClick,
            modifier = modifier,
            enabled = enabled,
            shape = androidx.compose.foundation.shape.CircleShape,
            color = androidx.compose.ui.graphics.Color.Transparent,
            contentColor = contentColor,
        ) {
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier.heightIn(min = 48.dp),
                contentAlignment = androidx.compose.ui.Alignment.CenterStart,
            ) {
                androidx.compose.foundation.layout.Box(
                    modifier = androidx.compose.ui.Modifier
                        .height(34.dp)
                        .background(compactFill, androidx.compose.foundation.shape.CircleShape)
                        .padding(horizontal = 12.dp),
                    contentAlignment = androidx.compose.ui.Alignment.Center,
                ) {
                    androidx.compose.material3.Text(
                        text,
                        style = androidx.compose.material3.MaterialTheme.typography.bodyLarge,
                        color = if (enabled) contentColor else contentColor.copy(alpha = 0.38f),
                        maxLines = 1,
                        softWrap = false,
                        overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                    )
                }
            }
        }
    }
}

@androidx.compose.runtime.Composable
private fun NexaDatePickerTimeInput(
    state: androidx.compose.material3.TimePickerState,
    style: String,
    colors: androidx.compose.material3.TimePickerColors,
    pickerSurface: androidx.compose.ui.graphics.Color,
    textColor: androidx.compose.ui.graphics.Color,
    accentColor: androidx.compose.ui.graphics.Color,
    selectedContentColor: androidx.compose.ui.graphics.Color,
    onInputValidityChanged: (Boolean) -> Unit,
) {
    if (style == "Input" || style == "Wheel") {
        // Material3 TimeInput requests focus on composition, which opens the IME before a tap.
        val is24Hour = state.is24hour
        val hourLabel = androidx.compose.ui.res.stringResource(id = androidx.compose.material3.R.string.m3c_time_picker_hour)
        val minuteLabel = androidx.compose.ui.res.stringResource(id = androidx.compose.material3.R.string.m3c_time_picker_minute)
        val hourDescription = androidx.compose.ui.res.stringResource(id = androidx.compose.material3.R.string.m3c_time_picker_hour_text_field)
        val minuteDescription = androidx.compose.ui.res.stringResource(id = androidx.compose.material3.R.string.m3c_time_picker_minute_text_field)
        val amLabel = androidx.compose.ui.res.stringResource(id = androidx.compose.material3.R.string.m3c_time_picker_am)
        val pmLabel = androidx.compose.ui.res.stringResource(id = androidx.compose.material3.R.string.m3c_time_picker_pm)
        var hourText by androidx.compose.runtime.remember(state, is24Hour) {
            androidx.compose.runtime.mutableStateOf(nexaFormatTimeInputHour(state.hour, is24Hour))
        }
        var minuteText by androidx.compose.runtime.remember(state) {
            androidx.compose.runtime.mutableStateOf(state.minute.toString().padStart(2, '0'))
        }
        var hourFocused by androidx.compose.runtime.remember(state) { androidx.compose.runtime.mutableStateOf(false) }
        var minuteFocused by androidx.compose.runtime.remember(state) { androidx.compose.runtime.mutableStateOf(false) }
        val fieldColors = androidx.compose.material3.OutlinedTextFieldDefaults.colors(
            focusedTextColor = textColor,
            unfocusedTextColor = textColor,
            focusedContainerColor = pickerSurface,
            unfocusedContainerColor = pickerSurface,
            focusedBorderColor = accentColor,
            unfocusedBorderColor = textColor.copy(alpha = 0.38f),
            focusedLabelColor = accentColor,
            unfocusedLabelColor = textColor.copy(alpha = 0.7f),
            cursorColor = accentColor,
        )
        val periodColors = androidx.compose.material3.FilterChipDefaults.filterChipColors(
            selectedContainerColor = accentColor,
            selectedLabelColor = selectedContentColor,
            labelColor = textColor,
        )
        androidx.compose.runtime.LaunchedEffect(state.hour, is24Hour, hourFocused) {
            if (!hourFocused) {
                hourText = nexaFormatTimeInputHour(state.hour, is24Hour)
                onInputValidityChanged(minuteText.toIntOrNull()?.let { it in 0..59 } == true)
            }
        }
        androidx.compose.runtime.LaunchedEffect(state.minute, minuteFocused) {
            if (!minuteFocused) {
                minuteText = state.minute.toString().padStart(2, '0')
                onInputValidityChanged(
                    hourText.toIntOrNull()?.let { if (is24Hour) it in 0..23 else it in 1..12 } == true,
                )
            }
        }
        androidx.compose.foundation.layout.Column(
            modifier = androidx.compose.ui.Modifier.fillMaxWidth().padding(16.dp),
            verticalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(8.dp),
        ) {
            androidx.compose.foundation.layout.Row(
                modifier = androidx.compose.ui.Modifier.fillMaxWidth(),
                verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(8.dp),
            ) {
                androidx.compose.material3.OutlinedTextField(
                    value = hourText,
                    onValueChange = { value ->
                        val nextHourText = value.filter(Char::isDigit).take(2)
                        hourText = nextHourText
                        val entered = nextHourText.toIntOrNull() ?: -1
                        val validHour = if (is24Hour) entered in 0..23 else entered in 1..12
                        val validMinute = minuteText.toIntOrNull()?.let { it in 0..59 } == true
                        onInputValidityChanged(validHour && validMinute)
                        if (validHour) {
                            val hour = if (is24Hour) entered else entered % 12
                            state.hour = hour + if (!is24Hour && state.hour >= 12) 12 else 0
                        }
                    },
                    modifier = androidx.compose.ui.Modifier
                        .weight(1f)
                        .onFocusChanged { hourFocused = it.isFocused }
                        .semantics { contentDescription = hourDescription },
                    label = { androidx.compose.material3.Text(hourLabel) },
                    singleLine = true,
                    isError = hourText.toIntOrNull()?.let { if (is24Hour) it !in 0..23 else it !in 1..12 } ?: true,
                    textStyle = androidx.compose.material3.MaterialTheme.typography.headlineSmall.copy(
                        textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                    ),
                    keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(
                        keyboardType = androidx.compose.ui.text.input.KeyboardType.Number,
                        imeAction = androidx.compose.ui.text.input.ImeAction.Done,
                    ),
                    colors = fieldColors,
                )
                androidx.compose.material3.Text(
                    text = ":",
                    color = textColor,
                    style = androidx.compose.material3.MaterialTheme.typography.headlineSmall,
                )
                androidx.compose.material3.OutlinedTextField(
                    value = minuteText,
                    onValueChange = { value ->
                        val nextMinuteText = value.filter(Char::isDigit).take(2)
                        minuteText = nextMinuteText
                        val entered = nextMinuteText.toIntOrNull() ?: -1
                        val validMinute = entered in 0..59
                        val validHour = hourText.toIntOrNull()?.let { if (is24Hour) it in 0..23 else it in 1..12 } == true
                        onInputValidityChanged(validHour && validMinute)
                        if (validMinute) state.minute = entered
                    },
                    modifier = androidx.compose.ui.Modifier
                        .weight(1f)
                        .onFocusChanged { minuteFocused = it.isFocused }
                        .semantics { contentDescription = minuteDescription },
                    label = { androidx.compose.material3.Text(minuteLabel) },
                    singleLine = true,
                    isError = minuteText.toIntOrNull()?.let { it !in 0..59 } ?: true,
                    textStyle = androidx.compose.material3.MaterialTheme.typography.headlineSmall.copy(
                        textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                    ),
                    keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(
                        keyboardType = androidx.compose.ui.text.input.KeyboardType.Number,
                        imeAction = androidx.compose.ui.text.input.ImeAction.Done,
                    ),
                    colors = fieldColors,
                )
            }
            if (!is24Hour) {
                val isPm = state.hour >= 12
                androidx.compose.foundation.layout.Row(
                    modifier = androidx.compose.ui.Modifier.align(androidx.compose.ui.Alignment.End),
                    horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(8.dp),
                ) {
                    androidx.compose.material3.FilterChip(
                        selected = !isPm,
                        onClick = { state.hour = state.hour % 12 },
                        colors = periodColors,
                        label = { androidx.compose.material3.Text(amLabel) },
                    )
                    androidx.compose.material3.FilterChip(
                        selected = isPm,
                        onClick = { state.hour = state.hour % 12 + 12 },
                        colors = periodColors,
                        label = { androidx.compose.material3.Text(pmLabel) },
                    )
                }
            }
        }
    } else {
        androidx.compose.material3.TimePicker(state = state, modifier = androidx.compose.ui.Modifier.fillMaxWidth(), colors = colors)
    }
}

private fun nexaFormatTimeInputHour(hour: Int, is24Hour: Boolean): String {
    val displayedHour = if (is24Hour) hour else (hour + 11) % 12 + 1
    return displayedHour.toString().padStart(2, '0')
}

private fun nexaPickerDateMillis(timestamp: Long, zone: java.util.TimeZone): Long {
    val local = java.util.Calendar.getInstance(zone).apply { timeInMillis = timestamp }
    return java.util.Calendar.getInstance(java.util.TimeZone.getTimeZone("UTC")).apply {
        clear()
        set(local.get(java.util.Calendar.YEAR), local.get(java.util.Calendar.MONTH), local.get(java.util.Calendar.DAY_OF_MONTH))
    }.timeInMillis
}

private fun nexaTimestampForPickerDate(dateMillis: Long, zone: java.util.TimeZone): Long {
    val utc = java.util.Calendar.getInstance(java.util.TimeZone.getTimeZone("UTC")).apply { timeInMillis = dateMillis }
    return java.util.Calendar.getInstance(zone).apply {
        clear()
        set(java.util.Calendar.YEAR, utc.get(java.util.Calendar.YEAR))
        set(java.util.Calendar.MONTH, utc.get(java.util.Calendar.MONTH))
        set(java.util.Calendar.DAY_OF_MONTH, utc.get(java.util.Calendar.DAY_OF_MONTH))
        set(java.util.Calendar.HOUR_OF_DAY, 0)
        set(java.util.Calendar.MINUTE, 0)
        set(java.util.Calendar.SECOND, 0)
        set(java.util.Calendar.MILLISECOND, 0)
    }.timeInMillis
}

private fun nexaTimestampForPickerDateTime(
    dateMillis: Long,
    hour: Int,
    minute: Int,
    zone: java.util.TimeZone,
): Long {
    val utc = java.util.Calendar.getInstance(java.util.TimeZone.getTimeZone("UTC")).apply { timeInMillis = dateMillis }
    return java.util.Calendar.getInstance(zone).apply {
        clear()
        set(java.util.Calendar.YEAR, utc.get(java.util.Calendar.YEAR))
        set(java.util.Calendar.MONTH, utc.get(java.util.Calendar.MONTH))
        set(java.util.Calendar.DAY_OF_MONTH, utc.get(java.util.Calendar.DAY_OF_MONTH))
        set(java.util.Calendar.HOUR_OF_DAY, hour)
        set(java.util.Calendar.MINUTE, minute)
        set(java.util.Calendar.SECOND, 0)
        set(java.util.Calendar.MILLISECOND, 0)
    }.timeInMillis
}

private fun nexaTimestampForPickerTime(
    reference: Long,
    hour: Int,
    minute: Int,
    zone: java.util.TimeZone,
    locale: java.util.Locale,
): Long = java.util.Calendar.getInstance(zone, locale).apply {
    timeInMillis = reference
    set(java.util.Calendar.HOUR_OF_DAY, hour)
    set(java.util.Calendar.MINUTE, minute)
    set(java.util.Calendar.SECOND, 0)
    set(java.util.Calendar.MILLISECOND, 0)
}.timeInMillis

private fun nexaFormatPickerValue(
    timestamp: Long,
    pickerType: String,
    locale: java.util.Locale,
    zone: java.util.TimeZone,
    is24Hour: Boolean?,
): String {
    val skeleton = when (pickerType) {
        "Time" -> if (is24Hour == true) "Hm" else "hm"
        "DateTime" -> if (is24Hour == true) "yMMMdHm" else "yMMMdhm"
        else -> "yMMMd"
    }
    val pattern = android.text.format.DateFormat.getBestDateTimePattern(locale, skeleton)
    return java.text.SimpleDateFormat(pattern, locale).apply { timeZone = zone }.format(java.util.Date(timestamp))
}

private fun nexaFormatDateRange(
    start: Long?,
    end: Long?,
    locale: java.util.Locale,
    zone: java.util.TimeZone,
    dateStyle: Int = java.text.DateFormat.MEDIUM,
): String {
    val format = java.text.DateFormat.getDateInstance(dateStyle, locale).apply { timeZone = zone }
    return listOfNotNull(start?.let { format.format(java.util.Date(it)) }, end?.let { format.format(java.util.Date(it)) })
        .joinToString(" – ")
        .ifEmpty { "Select dates" }
}

private fun nexaPickerTimeZone(identifier: String?): java.util.TimeZone {
    if (identifier.isNullOrBlank()) return java.util.TimeZone.getDefault()
    val requested = java.util.TimeZone.getTimeZone(identifier)
    return if (requested.id.equals(identifier, ignoreCase = true) ||
        java.util.TimeZone.getAvailableIDs().contains(identifier)
    ) {
        requested
    } else {
        java.util.TimeZone.getDefault()
    }
}

private fun nexaPickerLocale(identifier: String?): java.util.Locale {
    if (identifier.isNullOrBlank()) return java.util.Locale.getDefault()
    return java.util.Locale.forLanguageTag(identifier).takeIf { it.language.isNotBlank() }
        ?: java.util.Locale.getDefault()
}

private fun nexaPickerUses24HourClock(
    locale: java.util.Locale,
    localeIsExplicit: Boolean,
    context: android.content.Context,
): Boolean {
    if (!localeIsExplicit) return android.text.format.DateFormat.is24HourFormat(context)
    val pattern = android.text.format.DateFormat.getBestDateTimePattern(locale, "j")
    return pattern.any { it == 'H' || it == 'k' }
}

private fun nexaTimeHasSelectableMinute(
    reference: Long,
    minimum: Long?,
    maximum: Long?,
    zone: java.util.TimeZone,
): Boolean {
    if (minimum != null && maximum != null && minimum > maximum) return false
    if (minimum == null && maximum == null) return true
    val dayStart = java.util.Calendar.getInstance(zone).apply {
        timeInMillis = reference
        set(java.util.Calendar.HOUR_OF_DAY, 0)
        set(java.util.Calendar.MINUTE, 0)
        set(java.util.Calendar.SECOND, 0)
        set(java.util.Calendar.MILLISECOND, 0)
    }
    val nextDay = (dayStart.clone() as java.util.Calendar).apply { add(java.util.Calendar.DAY_OF_MONTH, 1) }
    val first = maxOf(minimum ?: Long.MIN_VALUE, dayStart.timeInMillis)
    val last = minOf(maximum ?: Long.MAX_VALUE, nextDay.timeInMillis - 1L)
    if (first > last) return false
    val firstMinute = nexaCeilToMinute(first, zone)
    val lastMinute = nexaFloorToMinute(last, zone)
    return firstMinute <= lastMinute
}

private fun nexaInstantRangeHasSelectableMinute(
    minimum: Long?,
    maximum: Long?,
    zone: java.util.TimeZone,
): Boolean {
    if (minimum != null && maximum != null && minimum > maximum) return false
    if (minimum == null || maximum == null) return true
    return nexaCeilToMinute(minimum, zone) <= nexaFloorToMinute(maximum, zone)
}

private fun nexaFloorToMinute(timestamp: Long, zone: java.util.TimeZone): Long =
    java.util.Calendar.getInstance(zone).apply {
        timeInMillis = timestamp
        set(java.util.Calendar.SECOND, 0)
        set(java.util.Calendar.MILLISECOND, 0)
    }.timeInMillis

private fun nexaCeilToMinute(timestamp: Long, zone: java.util.TimeZone): Long {
    val floor = nexaFloorToMinute(timestamp, zone)
    if (floor == timestamp) return floor
    return java.util.Calendar.getInstance(zone).apply {
        timeInMillis = floor
        add(java.util.Calendar.MINUTE, 1)
    }.timeInMillis
}

private fun nexaConstrainPickerTimestamp(
    value: Long,
    pickerType: String,
    minimum: Long?,
    maximum: Long?,
    zone: java.util.TimeZone,
    reference: Long,
): Long {
    if (minimum != null && maximum != null && minimum > maximum) return value
    return when (pickerType) {
        "Date" -> {
            val day = nexaPickerDateMillis(value, zone)
            val lower = minimum?.let { nexaPickerDateMillis(it, zone) } ?: Long.MIN_VALUE
            val upper = maximum?.let { nexaPickerDateMillis(it, zone) } ?: Long.MAX_VALUE
            nexaTimestampForPickerDate(day.coerceIn(lower, upper), zone)
        }
        "Time" -> {
            val dayStart = java.util.Calendar.getInstance(zone).apply {
                timeInMillis = reference
                set(java.util.Calendar.HOUR_OF_DAY, 0)
                set(java.util.Calendar.MINUTE, 0)
                set(java.util.Calendar.SECOND, 0)
                set(java.util.Calendar.MILLISECOND, 0)
            }
            val nextDay = (dayStart.clone() as java.util.Calendar).apply { add(java.util.Calendar.DAY_OF_MONTH, 1) }
            val first = maxOf(minimum ?: Long.MIN_VALUE, dayStart.timeInMillis)
            val last = minOf(maximum ?: Long.MAX_VALUE, nextDay.timeInMillis - 1L)
            if (first > last) value
            else nexaFloorToMinute(value, zone).coerceIn(nexaCeilToMinute(first, zone), nexaFloorToMinute(last, zone))
        }
        else -> nexaFloorToMinute(value, zone).coerceIn(
            minimum?.let { nexaCeilToMinute(it, zone) } ?: Long.MIN_VALUE,
            maximum?.let { nexaFloorToMinute(it, zone) } ?: Long.MAX_VALUE,
        )
    }
}
