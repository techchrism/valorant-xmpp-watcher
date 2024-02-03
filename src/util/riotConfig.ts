export interface RiotConfigResponse {
    'chat.affinities': {
        [key: string]: string
    }
    'chat.affinity_domains': {
        [key: string]: string
    }
}

async function getRiotClientConfig(token: string, entitlement: string): Promise<RiotConfigResponse> {
    return (await (await fetch('https://clientconfig.rpg.riotgames.com/api/v1/config/player?app=Riot%20Client', {
        headers: {
            'User-Agent': '',
            'Authorization': `Bearer ${token}`,
            'X-Riot-Entitlements-JWT': entitlement
        }
    })).json()) as RiotConfigResponse
}

let configCache: RiotConfigResponse | undefined = undefined

export async function getOrLoadRiotConfig(token: string, entitlement: string): Promise<RiotConfigResponse> {
    if(configCache !== undefined) return configCache
    configCache = await getRiotClientConfig(token, entitlement)
    return configCache
}