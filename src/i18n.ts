type Translations = typeof en;

const en = {
  recording: {
    title: 'Start Recording',
    meetingName: 'Meeting name',
    meetingNamePlaceholder: 'e.g. Q4 planning sync',
    meetingType: 'Meeting type',
    project: 'Project',
    noProject: 'No project',
    loadingProjects: 'Loading projects…',
    start: 'Start Recording',
    cancel: 'Cancel',
    notAuthenticated: 'Log in to start recording',
    notAuthenticatedDesc: 'Connect your Laconote account to capture and sync meetings.',
    login: 'Log In',
    permissionError: 'Permission required',
    openSettings: 'Open System Settings',
    sessionExpired: 'Session expired — please log in again.',
    loginAgain: 'Log In Again',
  },
  status: {
    title: 'Recording',
    chunks: (n: number) => `${n} chunk${n === 1 ? '' : 's'} uploaded`,
    stop: 'Stop Recording',
    stopping: 'Stopping…',
  },
  settings: {
    title: 'Settings',
    audio: 'Audio',
    account: 'Account',
    general: 'General',
    micDevice: 'Microphone',
    defaultMic: 'System Default',
    systemGain: 'System Audio Level',
    micGain: 'Microphone Level',
    loggedInAs: 'Logged in as',
    logout: 'Log Out',
    notLoggedIn: 'Not logged in',
    login: 'Log In',
    launchAtLogin: 'Launch at login',
    shortcutLabel: 'Toggle recording shortcut',
    close: 'Close',
    language: 'Language',
  },
  permissions: {
    title: 'Enable Recording Features',
    subtitle: 'To capture your meetings automatically, Laconote needs two permissions. These are standard macOS privacy controls — you can revoke them anytime.',
    whyTitle: 'Why we need this',
    benefitsTitle: 'What you get',
    systemAudio: 'System Audio',
    systemAudioDesc: 'Captures audio from Zoom, Meet, Teams, browser tabs, and any other app on your Mac — so remote participants are included in the transcript.',
    systemAudioBenefits: 'Remote participants captured · No extra setup · Works with any conferencing app',
    systemAudioWhy: 'macOS uses a privacy permission to let apps access audio from other applications. Laconote only reads audio — it never captures your screen.',
    microphone: 'Microphone',
    microphoneDesc: 'Captures your voice so your words appear in the transcript alongside everyone else on the call.',
    microphoneBenefits: 'Your voice in the transcript · Speaker identification · Full meeting context',
    microphoneWhy: 'Required to record your microphone input during meetings.',
    granted: 'Granted',
    notGranted: 'Not granted',
    needsVerification: 'Needs verification',
    openSettings: 'Open System Settings',
    grantMic: 'Grant Access',
    done: 'All set — Start recording',
    donePartial: 'Continue anyway',
    checkAgain: 'Check Again',
    settingsTab: 'Permissions',
    settingsTitle: 'Privacy & Permissions',
    settingsSubtitle: 'Control what Laconote can access on your Mac.',
    revokeNote: 'To revoke a permission, open System Settings → Privacy & Security.',
    allGranted: 'All permissions granted',
    someNotGranted: 'Some permissions are missing — recording may not work correctly.',
    restartRequired: 'After granting the permission in System Settings, restart Laconote to apply the change.',
    restartRequiredGranted: 'Permission granted. Please restart Laconote to apply changes.',
    devPermissionResetWarning: 'You are running a development build. macOS resets audio permissions when the binary changes (after each cargo rebuild). If permissions appear stuck: (1) re-grant in System Settings, (2) restart Laconote. For stable testing, use a signed release build: cargo tauri build.',
    forceRecheck: 'Re-check Permissions',
    restartApp: 'Restart Laconote',
  },
  shadow: {
    tab: 'Shadow Recording',
    bufferDuration: 'Buffer Duration',
    bufferDurationDesc: 'How much audio to keep in memory',
    minutes: 'min',
    scheduleEnabled: 'Auto-start on Schedule',
    scheduleStart: 'Start Time',
    scheduleEnd: 'End Time',
    activeDays: 'Active Days',
    weekdays: 'Weekdays',
    everyDay: 'Every Day',
    mon: 'Mon', tue: 'Tue', wed: 'Wed', thu: 'Thu', fri: 'Fri', sat: 'Sat', sun: 'Sun',
    saveShortcut: 'Quick-save shortcut',
  },
  types: {
    technical: 'Technical', standup: 'Standup', discussion: 'Discussion',
    general: 'General', interview: 'Interview', lead: 'Lead',
    delivery: 'Delivery', learning: 'Learning',
  } as Record<string, string>,
};

const uk: Translations = {
  recording: {
    title: 'Почати запис',
    meetingName: 'Назва зустрічі',
    meetingNamePlaceholder: 'напр. Планування Q4',
    meetingType: 'Тип зустрічі',
    project: 'Проект',
    noProject: 'Без проекту',
    loadingProjects: 'Завантаження проектів…',
    start: 'Почати запис',
    cancel: 'Скасувати',
    notAuthenticated: 'Увійдіть щоб почати запис',
    notAuthenticatedDesc: 'Підключіть обліковий запис Laconote для захоплення та синхронізації зустрічей.',
    login: 'Увійти',
    permissionError: 'Потрібен дозвіл',
    openSettings: 'Відкрити Системні налаштування',
    sessionExpired: 'Сесія закінчилась — увійдіть знову.',
    loginAgain: 'Увійти знову',
  },
  status: {
    title: 'Запис',
    chunks: (n: number) => `${n} фрагмент${n === 1 ? '' : n < 5 ? 'и' : 'ів'} завантажено`,
    stop: 'Зупинити запис',
    stopping: 'Зупиняємо…',
  },
  settings: {
    title: 'Налаштування',
    audio: 'Аудіо',
    account: 'Обліковий запис',
    general: 'Загальне',
    micDevice: 'Мікрофон',
    defaultMic: 'Системний за замовчуванням',
    systemGain: 'Рівень системного аудіо',
    micGain: 'Рівень мікрофону',
    loggedInAs: 'Авторизовано як',
    logout: 'Вийти',
    notLoggedIn: 'Не авторизовано',
    login: 'Увійти',
    launchAtLogin: 'Запускати при вході',
    shortcutLabel: 'Гаряча клавіша запису',
    close: 'Закрити',
    language: 'Мова',
  },
  permissions: {
    title: 'Увімкніть функції запису',
    subtitle: 'Для автоматичного захоплення зустрічей Laconote потребує двох дозволів. Це стандартні налаштування конфіденційності macOS — ви можете скасувати їх будь-коли.',
    whyTitle: 'Навіщо це потрібно',
    benefitsTitle: 'Що ви отримуєте',
    systemAudio: 'Системне аудіо',
    systemAudioDesc: 'Захоплює аудіо з Zoom, Meet, Teams, вкладок браузера та будь-якого іншого додатку — так що віддалені учасники будуть в транскрипті.',
    systemAudioBenefits: 'Віддалені учасники · Без додаткових налаштувань · Працює з будь-яким додатком',
    systemAudioWhy: 'macOS використовує дозвіл конфіденційності для доступу до аудіо інших додатків. Laconote тільки читає аудіо — ніколи не захоплює екран.',
    microphone: 'Мікрофон',
    microphoneDesc: 'Захоплює ваш голос, щоб ваші слова з\'явились в транскрипті поруч з усіма іншими учасниками.',
    microphoneBenefits: 'Ваш голос в транскрипті · Ідентифікація спікера · Повний контекст зустрічі',
    microphoneWhy: 'Потрібен для запису вашого мікрофону під час зустрічей.',
    granted: 'Надано',
    notGranted: 'Не надано',
    needsVerification: 'Потребує перевірки',
    openSettings: 'Відкрити Системні налаштування',
    grantMic: 'Надати доступ',
    done: 'Все готово — Почати запис',
    donePartial: 'Продовжити все одно',
    checkAgain: 'Перевірити знову',
    settingsTab: 'Дозволи',
    settingsTitle: 'Конфіденційність та дозволи',
    settingsSubtitle: 'Керуйте доступом Laconote на вашому Mac.',
    revokeNote: 'Щоб скасувати дозвіл, відкрийте Системні налаштування → Конфіденційність та безпека.',
    allGranted: 'Всі дозволи надані',
    someNotGranted: 'Деякі дозволи відсутні — запис може не працювати коректно.',
    restartRequired: 'Після надання дозволу в Системних налаштуваннях перезапустіть Laconote.',
    restartRequiredGranted: 'Дозвіл надано. Будь ласка, перезапустіть Laconote.',
    devPermissionResetWarning: 'Ви використовуєте збірку для розробки. macOS скидає дозволи аудіо при зміні бінарника. Якщо дозволи застрягли: (1) надайте знову в Системних налаштуваннях, (2) перезапустіть Laconote.',
    forceRecheck: 'Перевірити дозволи',
    restartApp: 'Перезапустити Laconote',
  },
  shadow: {
    tab: 'Тіньовий запис',
    bufferDuration: 'Тривалість буфера',
    bufferDurationDesc: 'Скільки аудіо тримати в пам\'яті',
    minutes: 'хв',
    scheduleEnabled: 'Автозапуск за розкладом',
    scheduleStart: 'Час початку',
    scheduleEnd: 'Час завершення',
    activeDays: 'Активні дні',
    weekdays: 'Робочі дні',
    everyDay: 'Кожен день',
    mon: 'Пн', tue: 'Вт', wed: 'Ср', thu: 'Чт', fri: 'Пт', sat: 'Сб', sun: 'Нд',
    saveShortcut: 'Швидке збереження',
  },
  types: {
    technical: 'Технічна', standup: 'Стендап', discussion: 'Обговорення',
    general: 'Загальна', interview: 'Інтерв\'ю', lead: 'Лід',
    delivery: 'Доставка', learning: 'Навчання',
  } as Record<string, string>,
};

const translations: Record<string, Translations> = { en, uk };

let currentLang = localStorage.getItem('laconote_lang') || 'en';

export function setLanguage(lang: string) {
  currentLang = lang;
  localStorage.setItem('laconote_lang', lang);
  window.dispatchEvent(new Event('laconote-lang-changed'));
}

export function getLanguage(): string {
  return currentLang;
}

export const AVAILABLE_LANGUAGES = [
  { code: 'en', label: 'English' },
  { code: 'uk', label: 'Українська' },
];

export const t: Translations = new Proxy(en, {
  get(_target, prop) {
    const lang = translations[currentLang] || en;
    return (lang as Record<string, unknown>)[prop as string];
  },
}) as Translations;

export const MEETING_TYPES = [
  'technical', 'standup', 'discussion', 'general',
  'interview', 'lead', 'delivery', 'learning',
] as const;

export type MeetingType = (typeof MEETING_TYPES)[number];
