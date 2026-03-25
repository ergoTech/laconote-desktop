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
    calendar: 'Calendar',
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
    calendar: 'Календар',
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

const ru: Translations = {
  recording: {
    title: 'Начать запись', meetingName: 'Название встречи', meetingNamePlaceholder: 'напр. Планирование Q4',
    meetingType: 'Тип встречи', project: 'Проект', noProject: 'Без проекта', loadingProjects: 'Загрузка проектов…',
    start: 'Начать запись', cancel: 'Отмена', notAuthenticated: 'Войдите чтобы начать запись',
    notAuthenticatedDesc: 'Подключите аккаунт Laconote для записи и синхронизации встреч.',
    login: 'Войти', permissionError: 'Требуется разрешение', openSettings: 'Открыть Системные настройки',
    sessionExpired: 'Сессия истекла — войдите снова.', loginAgain: 'Войти снова',
  },
  status: { title: 'Запись', chunks: (n: number) => `${n} фрагмент${n === 1 ? '' : n < 5 ? 'а' : 'ов'} загружено`, stop: 'Остановить запись', stopping: 'Останавливаем…' },
  settings: { title: 'Настройки', audio: 'Аудио', account: 'Аккаунт', general: 'Общее', micDevice: 'Микрофон', defaultMic: 'По умолчанию', systemGain: 'Уровень системного аудио', micGain: 'Уровень микрофона', loggedInAs: 'Авторизован как', logout: 'Выйти', notLoggedIn: 'Не авторизован', login: 'Войти', launchAtLogin: 'Запускать при входе', shortcutLabel: 'Горячая клавиша записи', close: 'Закрыть', language: 'Язык', calendar: 'Календарь' },
  permissions: { title: 'Включите функции записи', subtitle: 'Для автоматического захвата встреч Laconote нужны два разрешения. Это стандартные настройки macOS — вы можете отозвать их в любое время.', whyTitle: 'Зачем это нужно', benefitsTitle: 'Что вы получите', systemAudio: 'Системное аудио', systemAudioDesc: 'Захватывает аудио из Zoom, Meet, Teams, вкладок браузера и любого другого приложения.', systemAudioBenefits: 'Удалённые участники · Без настройки · Работает с любым приложением', systemAudioWhy: 'macOS требует разрешение для доступа к аудио других приложений.', microphone: 'Микрофон', microphoneDesc: 'Захватывает ваш голос для транскрипции.', microphoneBenefits: 'Ваш голос в транскрипте · Идентификация · Полный контекст', microphoneWhy: 'Требуется для записи микрофона.', granted: 'Предоставлено', notGranted: 'Не предоставлено', needsVerification: 'Требует проверки', openSettings: 'Открыть Системные настройки', grantMic: 'Предоставить доступ', done: 'Всё готово — Начать запись', donePartial: 'Продолжить всё равно', checkAgain: 'Проверить снова', settingsTab: 'Разрешения', settingsTitle: 'Конфиденциальность', settingsSubtitle: 'Управляйте доступом Laconote на Mac.', revokeNote: 'Чтобы отозвать разрешение, откройте Системные настройки → Конфиденциальность.', allGranted: 'Все разрешения предоставлены', someNotGranted: 'Некоторые разрешения отсутствуют.', restartRequired: 'После предоставления разрешения перезапустите Laconote.', restartRequiredGranted: 'Разрешение предоставлено. Перезапустите Laconote.', devPermissionResetWarning: 'Dev-сборка. macOS сбрасывает разрешения при пересборке.', forceRecheck: 'Проверить разрешения', restartApp: 'Перезапустить Laconote' },
  shadow: { tab: 'Теневая запись', bufferDuration: 'Длительность буфера', bufferDurationDesc: 'Сколько аудио хранить в памяти', minutes: 'мин', scheduleEnabled: 'Автозапуск по расписанию', scheduleStart: 'Начало', scheduleEnd: 'Конец', activeDays: 'Активные дни', weekdays: 'Будни', everyDay: 'Каждый день', mon: 'Пн', tue: 'Вт', wed: 'Ср', thu: 'Чт', fri: 'Пт', sat: 'Сб', sun: 'Вс', saveShortcut: 'Быстрое сохранение' },
  types: { technical: 'Техническая', standup: 'Стендап', discussion: 'Обсуждение', general: 'Общая', interview: 'Интервью', lead: 'Лид', delivery: 'Доставка', learning: 'Обучение' } as Record<string, string>,
};

const de: Translations = {
  recording: {
    title: 'Aufnahme starten', meetingName: 'Meeting-Name', meetingNamePlaceholder: 'z.B. Q4 Planung',
    meetingType: 'Meeting-Typ', project: 'Projekt', noProject: 'Kein Projekt', loadingProjects: 'Projekte laden…',
    start: 'Aufnahme starten', cancel: 'Abbrechen', notAuthenticated: 'Anmelden um aufzunehmen',
    notAuthenticatedDesc: 'Verbinden Sie Ihr Laconote-Konto zum Aufnehmen und Synchronisieren.',
    login: 'Anmelden', permissionError: 'Berechtigung erforderlich', openSettings: 'Systemeinstellungen öffnen',
    sessionExpired: 'Sitzung abgelaufen — bitte erneut anmelden.', loginAgain: 'Erneut anmelden',
  },
  status: { title: 'Aufnahme', chunks: (n: number) => `${n} Fragment${n === 1 ? '' : 'e'} hochgeladen`, stop: 'Aufnahme stoppen', stopping: 'Wird gestoppt…' },
  settings: { title: 'Einstellungen', audio: 'Audio', account: 'Konto', general: 'Allgemein', micDevice: 'Mikrofon', defaultMic: 'Systemstandard', systemGain: 'Systemaudio-Pegel', micGain: 'Mikrofon-Pegel', loggedInAs: 'Angemeldet als', logout: 'Abmelden', notLoggedIn: 'Nicht angemeldet', login: 'Anmelden', launchAtLogin: 'Bei Anmeldung starten', shortcutLabel: 'Aufnahme-Tastenkürzel', close: 'Schließen', language: 'Sprache', calendar: 'Kalender' },
  permissions: { title: 'Aufnahmefunktionen aktivieren', subtitle: 'Laconote benötigt zwei Berechtigungen für die automatische Meeting-Aufnahme.', whyTitle: 'Warum wird das benötigt', benefitsTitle: 'Was Sie erhalten', systemAudio: 'Systemaudio', systemAudioDesc: 'Erfasst Audio von Zoom, Meet, Teams und anderen Apps.', systemAudioBenefits: 'Remote-Teilnehmer · Keine Einrichtung · Jede App', systemAudioWhy: 'macOS erfordert eine Berechtigung für den Zugriff auf Audio anderer Apps.', microphone: 'Mikrofon', microphoneDesc: 'Erfasst Ihre Stimme für die Transkription.', microphoneBenefits: 'Ihre Stimme · Sprechererkennung · Vollständiger Kontext', microphoneWhy: 'Erforderlich für die Mikrofonaufnahme.', granted: 'Erteilt', notGranted: 'Nicht erteilt', needsVerification: 'Überprüfung nötig', openSettings: 'Systemeinstellungen öffnen', grantMic: 'Zugang gewähren', done: 'Alles bereit — Aufnahme starten', donePartial: 'Trotzdem fortfahren', checkAgain: 'Erneut prüfen', settingsTab: 'Berechtigungen', settingsTitle: 'Datenschutz & Berechtigungen', settingsSubtitle: 'Verwalten Sie den Zugriff von Laconote.', revokeNote: 'Berechtigung widerrufen: Systemeinstellungen → Datenschutz.', allGranted: 'Alle Berechtigungen erteilt', someNotGranted: 'Einige Berechtigungen fehlen.', restartRequired: 'Nach Erteilung bitte Laconote neu starten.', restartRequiredGranted: 'Berechtigung erteilt. Bitte Laconote neu starten.', devPermissionResetWarning: 'Dev-Build. macOS setzt Berechtigungen bei Neubau zurück.', forceRecheck: 'Berechtigungen prüfen', restartApp: 'Laconote neu starten' },
  shadow: { tab: 'Schattenaufnahme', bufferDuration: 'Pufferdauer', bufferDurationDesc: 'Wie viel Audio im Speicher behalten', minutes: 'Min', scheduleEnabled: 'Zeitplan-Autostart', scheduleStart: 'Startzeit', scheduleEnd: 'Endzeit', activeDays: 'Aktive Tage', weekdays: 'Werktage', everyDay: 'Jeden Tag', mon: 'Mo', tue: 'Di', wed: 'Mi', thu: 'Do', fri: 'Fr', sat: 'Sa', sun: 'So', saveShortcut: 'Schnellspeichern' },
  types: { technical: 'Technisch', standup: 'Standup', discussion: 'Diskussion', general: 'Allgemein', interview: 'Interview', lead: 'Lead', delivery: 'Lieferung', learning: 'Lernen' } as Record<string, string>,
};

const fr: Translations = {
  recording: {
    title: 'Démarrer l\'enregistrement', meetingName: 'Nom de la réunion', meetingNamePlaceholder: 'ex. Planification Q4',
    meetingType: 'Type de réunion', project: 'Projet', noProject: 'Aucun projet', loadingProjects: 'Chargement…',
    start: 'Démarrer l\'enregistrement', cancel: 'Annuler', notAuthenticated: 'Connectez-vous pour enregistrer',
    notAuthenticatedDesc: 'Connectez votre compte Laconote pour capturer et synchroniser les réunions.',
    login: 'Se connecter', permissionError: 'Autorisation requise', openSettings: 'Ouvrir les Préférences Système',
    sessionExpired: 'Session expirée — reconnectez-vous.', loginAgain: 'Se reconnecter',
  },
  status: { title: 'Enregistrement', chunks: (n: number) => `${n} fragment${n > 1 ? 's' : ''} téléchargé${n > 1 ? 's' : ''}`, stop: 'Arrêter l\'enregistrement', stopping: 'Arrêt…' },
  settings: { title: 'Paramètres', audio: 'Audio', account: 'Compte', general: 'Général', micDevice: 'Microphone', defaultMic: 'Par défaut', systemGain: 'Niveau audio système', micGain: 'Niveau microphone', loggedInAs: 'Connecté en tant que', logout: 'Déconnexion', notLoggedIn: 'Non connecté', login: 'Se connecter', launchAtLogin: 'Lancer au démarrage', shortcutLabel: 'Raccourci d\'enregistrement', close: 'Fermer', language: 'Langue', calendar: 'Calendrier' },
  permissions: { title: 'Activer les fonctions d\'enregistrement', subtitle: 'Laconote a besoin de deux autorisations pour capturer automatiquement les réunions.', whyTitle: 'Pourquoi c\'est nécessaire', benefitsTitle: 'Ce que vous obtenez', systemAudio: 'Audio système', systemAudioDesc: 'Capture l\'audio de Zoom, Meet, Teams et d\'autres apps.', systemAudioBenefits: 'Participants distants · Sans configuration · Toute app', systemAudioWhy: 'macOS nécessite une autorisation pour accéder à l\'audio d\'autres apps.', microphone: 'Microphone', microphoneDesc: 'Capture votre voix pour la transcription.', microphoneBenefits: 'Votre voix · Identification · Contexte complet', microphoneWhy: 'Requis pour enregistrer le microphone.', granted: 'Accordée', notGranted: 'Non accordée', needsVerification: 'Vérification nécessaire', openSettings: 'Ouvrir les Préférences Système', grantMic: 'Accorder l\'accès', done: 'Tout est prêt — Démarrer', donePartial: 'Continuer quand même', checkAgain: 'Vérifier à nouveau', settingsTab: 'Autorisations', settingsTitle: 'Confidentialité & Autorisations', settingsSubtitle: 'Gérez l\'accès de Laconote sur votre Mac.', revokeNote: 'Pour révoquer, ouvrez Préférences Système → Confidentialité.', allGranted: 'Toutes les autorisations accordées', someNotGranted: 'Certaines autorisations manquent.', restartRequired: 'Après avoir accordé l\'autorisation, redémarrez Laconote.', restartRequiredGranted: 'Autorisation accordée. Redémarrez Laconote.', devPermissionResetWarning: 'Build de dev. macOS réinitialise les autorisations après recompilation.', forceRecheck: 'Vérifier les autorisations', restartApp: 'Redémarrer Laconote' },
  shadow: { tab: 'Enregistrement fantôme', bufferDuration: 'Durée du tampon', bufferDurationDesc: 'Quantité d\'audio à garder en mémoire', minutes: 'min', scheduleEnabled: 'Démarrage auto programmé', scheduleStart: 'Heure de début', scheduleEnd: 'Heure de fin', activeDays: 'Jours actifs', weekdays: 'Jours ouvrables', everyDay: 'Chaque jour', mon: 'Lun', tue: 'Mar', wed: 'Mer', thu: 'Jeu', fri: 'Ven', sat: 'Sam', sun: 'Dim', saveShortcut: 'Sauvegarde rapide' },
  types: { technical: 'Technique', standup: 'Standup', discussion: 'Discussion', general: 'Général', interview: 'Entretien', lead: 'Lead', delivery: 'Livraison', learning: 'Formation' } as Record<string, string>,
};

const es: Translations = {
  recording: {
    title: 'Iniciar grabación', meetingName: 'Nombre de la reunión', meetingNamePlaceholder: 'ej. Planificación Q4',
    meetingType: 'Tipo de reunión', project: 'Proyecto', noProject: 'Sin proyecto', loadingProjects: 'Cargando…',
    start: 'Iniciar grabación', cancel: 'Cancelar', notAuthenticated: 'Inicia sesión para grabar',
    notAuthenticatedDesc: 'Conecta tu cuenta Laconote para capturar y sincronizar reuniones.',
    login: 'Iniciar sesión', permissionError: 'Permiso requerido', openSettings: 'Abrir Configuración del Sistema',
    sessionExpired: 'Sesión expirada — inicia sesión de nuevo.', loginAgain: 'Iniciar sesión de nuevo',
  },
  status: { title: 'Grabando', chunks: (n: number) => `${n} fragmento${n > 1 ? 's' : ''} subido${n > 1 ? 's' : ''}`, stop: 'Detener grabación', stopping: 'Deteniendo…' },
  settings: { title: 'Configuración', audio: 'Audio', account: 'Cuenta', general: 'General', micDevice: 'Micrófono', defaultMic: 'Predeterminado', systemGain: 'Nivel de audio del sistema', micGain: 'Nivel del micrófono', loggedInAs: 'Conectado como', logout: 'Cerrar sesión', notLoggedIn: 'No conectado', login: 'Iniciar sesión', launchAtLogin: 'Iniciar al arrancar', shortcutLabel: 'Atajo de grabación', close: 'Cerrar', language: 'Idioma', calendar: 'Calendario' },
  permissions: { title: 'Activar funciones de grabación', subtitle: 'Laconote necesita dos permisos para capturar reuniones automáticamente.', whyTitle: 'Por qué es necesario', benefitsTitle: 'Lo que obtienes', systemAudio: 'Audio del sistema', systemAudioDesc: 'Captura audio de Zoom, Meet, Teams y otras apps.', systemAudioBenefits: 'Participantes remotos · Sin configuración · Cualquier app', systemAudioWhy: 'macOS requiere permiso para acceder al audio de otras apps.', microphone: 'Micrófono', microphoneDesc: 'Captura tu voz para la transcripción.', microphoneBenefits: 'Tu voz · Identificación · Contexto completo', microphoneWhy: 'Requerido para grabar el micrófono.', granted: 'Concedido', notGranted: 'No concedido', needsVerification: 'Requiere verificación', openSettings: 'Abrir Configuración', grantMic: 'Conceder acceso', done: 'Todo listo — Iniciar grabación', donePartial: 'Continuar de todos modos', checkAgain: 'Verificar de nuevo', settingsTab: 'Permisos', settingsTitle: 'Privacidad y Permisos', settingsSubtitle: 'Gestiona el acceso de Laconote en tu Mac.', revokeNote: 'Para revocar, abre Configuración → Privacidad.', allGranted: 'Todos los permisos concedidos', someNotGranted: 'Faltan algunos permisos.', restartRequired: 'Después de conceder permisos, reinicia Laconote.', restartRequiredGranted: 'Permiso concedido. Reinicia Laconote.', devPermissionResetWarning: 'Build de desarrollo. macOS reinicia permisos al recompilar.', forceRecheck: 'Verificar permisos', restartApp: 'Reiniciar Laconote' },
  shadow: { tab: 'Grabación en sombra', bufferDuration: 'Duración del búfer', bufferDurationDesc: 'Cuánto audio mantener en memoria', minutes: 'min', scheduleEnabled: 'Inicio automático programado', scheduleStart: 'Hora de inicio', scheduleEnd: 'Hora de fin', activeDays: 'Días activos', weekdays: 'Días laborables', everyDay: 'Cada día', mon: 'Lun', tue: 'Mar', wed: 'Mié', thu: 'Jue', fri: 'Vie', sat: 'Sáb', sun: 'Dom', saveShortcut: 'Guardado rápido' },
  types: { technical: 'Técnica', standup: 'Standup', discussion: 'Discusión', general: 'General', interview: 'Entrevista', lead: 'Lead', delivery: 'Entrega', learning: 'Aprendizaje' } as Record<string, string>,
};

const it: Translations = {
  recording: {
    title: 'Avvia registrazione', meetingName: 'Nome riunione', meetingNamePlaceholder: 'es. Pianificazione Q4',
    meetingType: 'Tipo di riunione', project: 'Progetto', noProject: 'Nessun progetto', loadingProjects: 'Caricamento…',
    start: 'Avvia registrazione', cancel: 'Annulla', notAuthenticated: 'Accedi per registrare',
    notAuthenticatedDesc: 'Collega il tuo account Laconote per catturare e sincronizzare le riunioni.',
    login: 'Accedi', permissionError: 'Permesso necessario', openSettings: 'Apri Impostazioni di Sistema',
    sessionExpired: 'Sessione scaduta — accedi di nuovo.', loginAgain: 'Accedi di nuovo',
  },
  status: { title: 'Registrazione', chunks: (n: number) => `${n} frammento${n > 1 ? 'i' : ''} caricato${n > 1 ? 'i' : ''}`, stop: 'Ferma registrazione', stopping: 'Arresto…' },
  settings: { title: 'Impostazioni', audio: 'Audio', account: 'Account', general: 'Generale', micDevice: 'Microfono', defaultMic: 'Predefinito', systemGain: 'Livello audio sistema', micGain: 'Livello microfono', loggedInAs: 'Connesso come', logout: 'Disconnetti', notLoggedIn: 'Non connesso', login: 'Accedi', launchAtLogin: 'Avvia all\'accesso', shortcutLabel: 'Scorciatoia registrazione', close: 'Chiudi', language: 'Lingua', calendar: 'Calendario' },
  permissions: { title: 'Attiva le funzioni di registrazione', subtitle: 'Laconote necessita di due permessi per catturare automaticamente le riunioni.', whyTitle: 'Perché è necessario', benefitsTitle: 'Cosa ottieni', systemAudio: 'Audio di sistema', systemAudioDesc: 'Cattura audio da Zoom, Meet, Teams e altre app.', systemAudioBenefits: 'Partecipanti remoti · Senza configurazione · Qualsiasi app', systemAudioWhy: 'macOS richiede un permesso per accedere all\'audio di altre app.', microphone: 'Microfono', microphoneDesc: 'Cattura la tua voce per la trascrizione.', microphoneBenefits: 'La tua voce · Identificazione · Contesto completo', microphoneWhy: 'Necessario per registrare il microfono.', granted: 'Concesso', notGranted: 'Non concesso', needsVerification: 'Verifica necessaria', openSettings: 'Apri Impostazioni', grantMic: 'Concedi accesso', done: 'Tutto pronto — Avvia', donePartial: 'Continua comunque', checkAgain: 'Verifica di nuovo', settingsTab: 'Permessi', settingsTitle: 'Privacy e Permessi', settingsSubtitle: 'Gestisci l\'accesso di Laconote.', revokeNote: 'Per revocare, apri Impostazioni → Privacy.', allGranted: 'Tutti i permessi concessi', someNotGranted: 'Alcuni permessi mancano.', restartRequired: 'Dopo aver concesso il permesso, riavvia Laconote.', restartRequiredGranted: 'Permesso concesso. Riavvia Laconote.', devPermissionResetWarning: 'Build di sviluppo. macOS resetta i permessi alla ricompilazione.', forceRecheck: 'Verifica permessi', restartApp: 'Riavvia Laconote' },
  shadow: { tab: 'Registrazione ombra', bufferDuration: 'Durata buffer', bufferDurationDesc: 'Quanto audio mantenere in memoria', minutes: 'min', scheduleEnabled: 'Avvio automatico programmato', scheduleStart: 'Ora di inizio', scheduleEnd: 'Ora di fine', activeDays: 'Giorni attivi', weekdays: 'Giorni feriali', everyDay: 'Ogni giorno', mon: 'Lun', tue: 'Mar', wed: 'Mer', thu: 'Gio', fri: 'Ven', sat: 'Sab', sun: 'Dom', saveShortcut: 'Salvataggio rapido' },
  types: { technical: 'Tecnica', standup: 'Standup', discussion: 'Discussione', general: 'Generale', interview: 'Colloquio', lead: 'Lead', delivery: 'Consegna', learning: 'Formazione' } as Record<string, string>,
};

const pl: Translations = {
  recording: {
    title: 'Rozpocznij nagrywanie', meetingName: 'Nazwa spotkania', meetingNamePlaceholder: 'np. Planowanie Q4',
    meetingType: 'Typ spotkania', project: 'Projekt', noProject: 'Brak projektu', loadingProjects: 'Ładowanie…',
    start: 'Rozpocznij nagrywanie', cancel: 'Anuluj', notAuthenticated: 'Zaloguj się aby nagrywać',
    notAuthenticatedDesc: 'Połącz konto Laconote aby przechwytywać i synchronizować spotkania.',
    login: 'Zaloguj się', permissionError: 'Wymagane uprawnienie', openSettings: 'Otwórz Ustawienia Systemowe',
    sessionExpired: 'Sesja wygasła — zaloguj się ponownie.', loginAgain: 'Zaloguj ponownie',
  },
  status: { title: 'Nagrywanie', chunks: (n: number) => `${n} fragment${n === 1 ? '' : n < 5 ? 'y' : 'ów'} przesłano`, stop: 'Zatrzymaj nagrywanie', stopping: 'Zatrzymywanie…' },
  settings: { title: 'Ustawienia', audio: 'Audio', account: 'Konto', general: 'Ogólne', micDevice: 'Mikrofon', defaultMic: 'Domyślny', systemGain: 'Poziom dźwięku systemu', micGain: 'Poziom mikrofonu', loggedInAs: 'Zalogowany jako', logout: 'Wyloguj', notLoggedIn: 'Niezalogowany', login: 'Zaloguj się', launchAtLogin: 'Uruchom przy logowaniu', shortcutLabel: 'Skrót nagrywania', close: 'Zamknij', language: 'Język', calendar: 'Kalendarz' },
  permissions: { title: 'Włącz funkcje nagrywania', subtitle: 'Laconote potrzebuje dwóch uprawnień do automatycznego przechwytywania spotkań.', whyTitle: 'Dlaczego to potrzebne', benefitsTitle: 'Co otrzymujesz', systemAudio: 'Dźwięk systemowy', systemAudioDesc: 'Przechwytuje dźwięk z Zoom, Meet, Teams i innych aplikacji.', systemAudioBenefits: 'Zdalni uczestnicy · Bez konfiguracji · Dowolna aplikacja', systemAudioWhy: 'macOS wymaga uprawnienia do dźwięku innych aplikacji.', microphone: 'Mikrofon', microphoneDesc: 'Przechwytuje Twój głos do transkrypcji.', microphoneBenefits: 'Twój głos · Identyfikacja · Pełny kontekst', microphoneWhy: 'Wymagane do nagrywania mikrofonu.', granted: 'Przyznane', notGranted: 'Nie przyznane', needsVerification: 'Wymaga weryfikacji', openSettings: 'Otwórz Ustawienia', grantMic: 'Przyznaj dostęp', done: 'Wszystko gotowe — Rozpocznij', donePartial: 'Kontynuuj mimo to', checkAgain: 'Sprawdź ponownie', settingsTab: 'Uprawnienia', settingsTitle: 'Prywatność i Uprawnienia', settingsSubtitle: 'Zarządzaj dostępem Laconote.', revokeNote: 'Aby odwołać, otwórz Ustawienia → Prywatność.', allGranted: 'Wszystkie uprawnienia przyznane', someNotGranted: 'Brakuje niektórych uprawnień.', restartRequired: 'Po przyznaniu uprawnień uruchom ponownie Laconote.', restartRequiredGranted: 'Uprawnienie przyznane. Uruchom ponownie Laconote.', devPermissionResetWarning: 'Build deweloperski. macOS resetuje uprawnienia po rekompilacji.', forceRecheck: 'Sprawdź uprawnienia', restartApp: 'Uruchom ponownie Laconote' },
  shadow: { tab: 'Nagrywanie w tle', bufferDuration: 'Czas trwania bufora', bufferDurationDesc: 'Ile audio trzymać w pamięci', minutes: 'min', scheduleEnabled: 'Automatyczny start wg harmonogramu', scheduleStart: 'Godzina rozpoczęcia', scheduleEnd: 'Godzina zakończenia', activeDays: 'Aktywne dni', weekdays: 'Dni robocze', everyDay: 'Każdy dzień', mon: 'Pon', tue: 'Wt', wed: 'Śr', thu: 'Czw', fri: 'Pt', sat: 'Sob', sun: 'Ndz', saveShortcut: 'Szybki zapis' },
  types: { technical: 'Techniczna', standup: 'Standup', discussion: 'Dyskusja', general: 'Ogólna', interview: 'Rozmowa', lead: 'Lead', delivery: 'Dostawa', learning: 'Nauka' } as Record<string, string>,
};

const translations: Record<string, Translations> = { en, uk, ru, de, fr, es, it, pl };

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
  { code: 'ru', label: 'Русский' },
  { code: 'de', label: 'Deutsch' },
  { code: 'fr', label: 'Français' },
  { code: 'es', label: 'Español' },
  { code: 'it', label: 'Italiano' },
  { code: 'pl', label: 'Polski' },
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
