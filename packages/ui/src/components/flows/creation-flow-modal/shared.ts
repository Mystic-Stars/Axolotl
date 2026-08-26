export type { GameVersionType } from './version-types'
export { aprilFoolsVersions, isVersionTypeMatch } from './version-types'
export { formatLoaderLabel, loaderDisplayNames } from '#ui/utils/loaders'

export const capitalize = (item: string) => item.charAt(0).toUpperCase() + item.slice(1)
