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
            jwtCookie.value = `Bearer ${response.JWT}`;

            navigateTo(`/${response.sessionId}`);
        });
}