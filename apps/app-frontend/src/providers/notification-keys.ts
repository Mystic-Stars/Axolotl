import type { PopupNotification, WebNotification } from '@modrinth/ui'

export function webNotificationKey(notification: WebNotification): string {
    return JSON.stringify([
        notification.title ?? '',
        notification.text ?? '',
        notification.type ?? '',
        notification.errorCode ?? '',
    ])
}

export function popupNotificationKey(notification: PopupNotification): string {
    return JSON.stringify([
        notification.title,
        notification.text ?? '',
        notification.type ?? '',
        notification.toast?.type ?? '',
        notification.toast?.actorName ?? '',
        notification.toast?.entityName ?? '',
        notification.progressItems
            ? notification.progressItems
                  .map((item) => item.id)
                  .sort()
                  .join('|')
            : '',
    ])
}

export function legacyPopupNotificationKey(notification: PopupNotification): string {
    return JSON.stringify([notification.title, notification.text ?? '', notification.type ?? ''])
}
