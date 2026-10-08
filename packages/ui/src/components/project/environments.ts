import type { Labrinth } from '@modrinth/api-client'
import { ClientIcon, ServerIcon, UserIcon } from '@modrinth/assets'
import type { Component } from 'vue'

import { defineMessage, type MessageDescriptor } from '../../composables/i18n'

const environmentTagLabels = {
    clientSide: defineMessage({
        id: 'project.about.compatibility.environments.client-side',
        defaultMessage: 'Client-side',
    }),
    serverSide: defineMessage({
        id: 'project.about.compatibility.environments.server-side',
        defaultMessage: 'Server-side',
    }),
    dedicatedServersOnly: defineMessage({
        id: 'project.about.compatibility.environments.dedicated-servers-only',
        defaultMessage: 'Dedicated servers only',
    }),
    singleplayerOnly: defineMessage({
        id: 'project.about.compatibility.environments.singleplayer-only',
        defaultMessage: 'Singleplayer only',
    }),
    singleplayer: defineMessage({
        id: 'project.about.compatibility.environments.singleplayer',
        defaultMessage: 'Singleplayer',
    }),
    clientAndServer: defineMessage({
        id: 'project.about.compatibility.environments.client-and-server',
        defaultMessage: 'Client and server',
    }),
    unknown: defineMessage({
        id: 'project.environment.tag.unknown',
        defaultMessage: 'Unknown',
    }),
    notApplicable: defineMessage({
        id: 'project.environment.tag.not-applicable',
        defaultMessage: 'N/A',
    }),
} as const

export function getEnvironmentTags(
    environment?: Labrinth.Projects.v3.Environment,
): Array<{ icon: Component | null; label: MessageDescriptor }> {
    switch (environment) {
        case 'client_only':
            return [{ icon: ClientIcon, label: environmentTagLabels.clientSide }]

        case 'server_only':
            return [
                { icon: ServerIcon, label: environmentTagLabels.serverSide },
                { icon: UserIcon, label: environmentTagLabels.singleplayer },
            ]

        case 'singleplayer_only':
            return [{ icon: UserIcon, label: environmentTagLabels.singleplayerOnly }]

        case 'dedicated_server_only':
            return [{ icon: ServerIcon, label: environmentTagLabels.dedicatedServersOnly }]

        case 'client_and_server':
            return [{ icon: ClientIcon, label: environmentTagLabels.clientAndServer }]

        case 'client_only_server_optional':
            return [
                { icon: ClientIcon, label: environmentTagLabels.clientSide },
                { icon: ClientIcon, label: environmentTagLabels.clientAndServer },
            ]

        case 'server_only_client_optional':
            return [
                { icon: ServerIcon, label: environmentTagLabels.serverSide },
                { icon: ClientIcon, label: environmentTagLabels.clientAndServer },
            ]

        case 'client_or_server':
            return [
                { icon: ClientIcon, label: environmentTagLabels.clientSide },
                { icon: ServerIcon, label: environmentTagLabels.serverSide },
            ]

        case 'client_or_server_prefers_both':
            return [
                { icon: ClientIcon, label: environmentTagLabels.clientSide },
                { icon: ServerIcon, label: environmentTagLabels.serverSide },
                { icon: ClientIcon, label: environmentTagLabels.clientAndServer },
            ]

        case 'unknown':
            return [{ label: environmentTagLabels.unknown, icon: null }]

        default:
            return [{ label: environmentTagLabels.notApplicable, icon: null }]
    }
}
