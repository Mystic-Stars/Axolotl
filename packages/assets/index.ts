/**
 * NOTE: You should re-export any manually added icons
 *       using consts to help TypeScript resolve the proper type
 *
 * NOTE: If an icon is part of the lucide icon set, it should be placed in the "icons" folder
 *       and automatically generated through the "pnpm run fix" command.
 */

import './omorphia.scss'

// Branding
import _ModrinthIcon from './branding/logo.svg?component'
// External Icons
import _AppleIcon from './external/apple.svg?component'
import _BlueskyIcon from './external/bluesky.svg?component'
import _BuyMeACoffeeIcon from './external/bmac.svg?component'
import _USDCColorIcon from './external/color/usdc.svg?component'
import _CurseForgeIcon from './external/curseforge.svg?component'
import _DiscordIcon from './external/discord.svg?component'
import _FacebookIcon from './external/facebook.svg?component'
import _GithubIcon from './external/github.svg?component'
import _IntercomBubbleIcon from './external/illustrations/intercom_bubble_icon.png?url'
import _MinecraftServerIcon from './external/illustrations/minecraft_server_icon.png?url'
import _InstagramIcon from './external/instagram.svg?component'
import _KoFiIcon from './external/kofi.svg?component'
import _MastodonIcon from './external/mastodon.svg?component'
import _OpenCollectiveIcon from './external/opencollective.svg?component'
import _PatreonIcon from './external/patreon.svg?component'
import _PayPalIcon from './external/paypal.svg?component'
import _PolygonIcon from './external/polygon.svg?component'
import _RedditIcon from './external/reddit.svg?component'
import _ReelsIcon from './external/reels.svg?component'
import _SnapchatIcon from './external/snapchat.svg?component'
import _ThreadsIcon from './external/threads.svg?component'
import _TikTokIcon from './external/tiktok.svg?component'
import _TumblrIcon from './external/tumblr.svg?component'
import _TwitchIcon from './external/twitch.svg?component'
import _TwitterIcon from './external/twitter.svg?component'
import _WindowsIcon from './external/windows.svg?component'
import _YouTubeIcon from './external/youtube.svg?component'
import _YouTubeGaming from './external/youtubegaming.svg?component'
import _YouTubeShortsIcon from './external/youtubeshorts.svg?component'
import _LinuxIcon from './external/linux.svg?component'
// Tag icon helpers - import maps from generated-icons
import type { IconComponent } from './generated-icons'
import { categoryIconMap, loaderIconMap } from './generated-icons'
import _DoneIllustration from './illustrations/done.svg?component'
import _EmptyIllustration from './illustrations/empty.svg?component'
import _EmptyInboxIllustration from './illustrations/empty-inbox.svg?component'
import _ErrorIllustration from './illustrations/error.svg?component'
import _NoConnectionIllustration from './illustrations/no-connection.svg?component'
import _NoCreditCardIllustration from './illustrations/no-credit-card.svg?component'
import _NoDocumentsIllustration from './illustrations/no-documents.svg?component'
import _NoGPSIllustration from './illustrations/no-gps.svg?component'
import _NoImagesIllustration from './illustrations/no-images.svg?component'
import _NoItemsCartIllustration from './illustrations/no-items-cart.svg?component'
import _NoMessagesIllustration from './illustrations/no-messages.svg?component'
import _NoSearchResultIllustration from './illustrations/no-search-result.svg?component'
import _NoTasksIllustration from './illustrations/no-tasks.svg?component'

export const ModrinthIcon = _ModrinthIcon
export const AppleIcon = _AppleIcon
export const BlueskyIcon = _BlueskyIcon
export const BuyMeACoffeeIcon = _BuyMeACoffeeIcon
export const GithubIcon = _GithubIcon
export const CurseForgeIcon = _CurseForgeIcon
export const DiscordIcon = _DiscordIcon
export const FacebookIcon = _FacebookIcon
export const InstagramIcon = _InstagramIcon
export const SnapchatIcon = _SnapchatIcon
export const ReelsIcon = _ReelsIcon
export const TikTokIcon = _TikTokIcon
export const TwitchIcon = _TwitchIcon
export const ThreadsIcon = _ThreadsIcon
export const KoFiIcon = _KoFiIcon
export const MastodonIcon = _MastodonIcon
export const OpenCollectiveIcon = _OpenCollectiveIcon
export const PatreonIcon = _PatreonIcon
export const PayPalIcon = _PayPalIcon
export const RedditIcon = _RedditIcon
export const TumblrIcon = _TumblrIcon
export const TwitterIcon = _TwitterIcon
export const WindowsIcon = _WindowsIcon
export const YouTubeIcon = _YouTubeIcon
export const YouTubeGaming = _YouTubeGaming
export const YouTubeShortsIcon = _YouTubeShortsIcon
export const PolygonIcon = _PolygonIcon
export const USDCColorIcon = _USDCColorIcon
export const IntercomBubbleIcon = _IntercomBubbleIcon
export const MinecraftServerIcon = _MinecraftServerIcon
export const LinuxIcon = _LinuxIcon

export * from './generated-icons'
export { default as ClassicPlayerModel } from './models/classic-player.gltf?url'
export { default as SlimPlayerModel } from './models/slim-player.gltf?url'

export const DoneIllustration = _DoneIllustration
export const EmptyIllustration = _EmptyIllustration
export const EmptyInboxIllustration = _EmptyInboxIllustration
export const ErrorIllustration = _ErrorIllustration
export const NoConnectionIllustration = _NoConnectionIllustration
export const NoCreditCardIllustration = _NoCreditCardIllustration
export const NoDocumentsIllustration = _NoDocumentsIllustration
export const NoGPSIllustration = _NoGPSIllustration
export const NoImagesIllustration = _NoImagesIllustration
export const NoItemsCartIllustration = _NoItemsCartIllustration
export const NoMessagesIllustration = _NoMessagesIllustration
export const NoSearchResultIllustration = _NoSearchResultIllustration
export const NoTasksIllustration = _NoTasksIllustration

export function getCategoryIcon(categoryName: string): IconComponent | undefined {
    if (!categoryName) {
        return undefined
    }
    return categoryIconMap[categoryName.toLowerCase()]
}

export function getLoaderIcon(loaderName: string): IconComponent | undefined {
    if (!loaderName) {
        return undefined
    }
    return loaderIconMap[loaderName.toLowerCase()]
}

// will try loader first, then category
export function getTagIcon(tagName: string): IconComponent | undefined {
    if (!tagName) {
        return undefined
    }
    return getLoaderIcon(tagName) ?? getCategoryIcon(tagName)
}

export const SERVER_CATEGORY_ICON_MAP: Record<string, string> = {
    'adventure-mode': 'compass',
    anarchy: 'skull',
    'battle-royale': 'target',
    bedwars: 'bed-double',
    bosses: 'crown',
    classes: 'badge',
    competitive: 'trophy',
    'creative-mode': 'palette',
    'creator-community': 'clapperboard',
    crossplay: 'gamepad-2',
    'custom-content': 'blocks',
    dungeons: 'castle',
    factions: 'flag',
    gens: 'pickaxe',
    'hardcore-mode': 'heart-crack',
    'keep-inventory': 'backpack',
    kitpvp: 'sword',
    lifesteal: 'heart-pulse',
    media: 'film',
    microgames: 'grid-3x3',
    minigames: 'dices',
    mmo: 'globe',
    network: 'network',
    'offline-mode': 'wifi-off',
    oneblock: 'square',
    op: 'zap',
    parkour: 'footprints',
    'personal-worlds': 'house',
    plots: 'map-pinned',
    pokemon: 'paw-print',
    prison: 'lock',
    pve: 'shield',
    pvp: 'swords',
    questing: 'scroll-text',
    racing: 'gauge',
    'recording-smp': 'camera',
    roleplay: 'theater',
    rpg: 'wand-sparkles',
    skyblock: 'cloud',
    smp: 'users',
    'survival-mode': 'tree-pine',
    teams: 'handshake',
    technical: 'terminal',
    towns: 'building-2',
    whitelisted: 'badge-check',
    'world-resets': 'refresh-ccw',
}
