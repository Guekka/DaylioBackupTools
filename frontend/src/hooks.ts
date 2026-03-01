import {deLocalizeUrl} from '$lib/paraglide/runtime';

export const reroute = (request: any) => deLocalizeUrl(request.url).pathname;
