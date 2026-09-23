import type { Labrinth } from '@modrinth/api-client'
import type { Ref } from 'vue'

import { createContext } from './create-context'

export interface TagsContext {
	gameVersions: Ref<Labrinth.Tags.v2.GameVersion[]>
	loaders: Ref<Labrinth.Tags.v2.Loader[]>
	/** Re-fetch tags so newly published game versions appear without an app restart. */
	refreshGameVersions?: () => Promise<void>
}

export const [injectTags, provideTags] = createContext<TagsContext>('root', 'tags')
