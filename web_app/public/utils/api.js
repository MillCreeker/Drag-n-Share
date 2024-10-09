export function createSession() {
    const config = useRuntimeConfig();

    $fetch(`${config.public.apiUri}/session`, {
        method: "POST",
    })
        .then((res) => {
            const results = JSON.parse(res);

            if (!results.success) {
                // TODO message
                return;
            }

            const response = results.response;

            const jwtCookie = useCookie('jwt');
            jwtCookie.value = response.jwt;

            navigateTo(`/${response.sessionId}`);
        });
}

export async function getIdForSessionName(sessionName) {
    const config = useRuntimeConfig();

    const res = await $fetch(`${config.public.apiUri}/idForName/${sessionName}`);

    const results = JSON.parse(res);

    if (!results.success) throw 'unsuccessful';

    const response = results.response;
    return response.sessionId;
}