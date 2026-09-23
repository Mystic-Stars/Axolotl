import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

const thumbnailComposable = readFileSync(
	new URL('../../../composables/use-image-thumbnail.ts', import.meta.url),
	'utf8',
)
const card = readFileSync(new URL('./card.vue', import.meta.url), 'utf8')
const screenshotsPage = readFileSync(new URL('./index.vue', import.meta.url), 'utf8')

test('screenshot cards request bounded backend thumbnails instead of original files', () => {
	assert.match(thumbnailComposable, /plugin:files\|screenshot_thumbnail/)
	assert.match(thumbnailComposable, /MAX_CONCURRENT_THUMBNAILS = 4/)
	assert.match(card, /filePath: `screenshots\/\$\{props\.screenshot\.file_name\}`/)
	assert.match(card, /const thumbnail = useImageThumbnail\(/)
})

test('full screenshot URLs remain reserved for the image viewer', () => {
	assert.match(screenshotsPage, /src: screenshot\.url/)
	assert.doesNotMatch(card, /:src="screenshot\.url"/)
})
