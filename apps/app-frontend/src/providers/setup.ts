import type { AbstractPopupNotificationManager, AbstractWebNotificationManager } from '@modrinth/ui'

import { setupCreationModal } from './setup/creation-modal'
import { setupFileDropProvider } from './setup/file-drop'
import { setupFilePickerProvider } from './setup/file-picker'
import { setupImageViewerEditorProvider } from './setup/image-viewer-editor'
import { setupInstanceImportProvider } from './setup/instance-import'
import { setupTagsProvider } from './setup/tags'

export function setupProviders(
	notificationManager: AbstractWebNotificationManager,
	popupNotificationManager: AbstractPopupNotificationManager,
	stateInitialization: Promise<void>,
) {
	setupTagsProvider(notificationManager, stateInitialization)
	const fileDrop = setupFileDropProvider()
	const filePicker = setupFilePickerProvider()
	setupImageViewerEditorProvider()
	setupInstanceImportProvider(notificationManager)

	return {
		fileDrop,
		...filePicker,
		...setupCreationModal(notificationManager, popupNotificationManager),
	}
}
