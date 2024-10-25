import { nextTick } from "vue";

export function createSession(files) {
    const config = useRuntimeConfig();

    $fetch(`${config.public.apiUri}/session`, {
        method: "POST",
        server: false
    })
        .then(async (res) => {
            const results = JSON.parse(res);

            if (!results.success) {
                // TODO message
                return;
            }

            const response = results.response;

            const jwtCookie = useCookie('jwt');
            jwtCookie.value = response.jwt;

            if (files) {
                await nextTick(); // ensure cookie is assigned
                await uploadFiles(files, response.sessionId);
            }

            navigateTo(`/${response.sessionId}`);
        });
}

export async function getIdForSessionName(sessionName) {
    const config = useRuntimeConfig();

    const res = await $fetch(`${config.public.apiUri}/idForName/${sessionName}`, {
        server: false
    });

    const results = JSON.parse(res);

    if (!results.success) throw 'unsuccessful';

    const response = results.response;
    return response.sessionId;
}

export async function uploadFiles(files, sessionId) {
    const config = useRuntimeConfig();

    const jwtCookie = useCookie('jwt');

    const body = files.map(f => {
        return {
            name: f.name,
            size: f.size
        };
    });

    try {
        await $fetch(`${config.public.apiUri}/files/${sessionId}`, {
            method: 'POST',
            headers: {
                Authorization: `Bearer ${jwtCookie.value}`
            },
            body: body,
            server: false
        });

        for (let i = 0; i < files.length; i++) {
            const file = files[i];
            const fileCookie = useCookie(`file-${file.name}`);
            fileCookie.value = file.data;
        }
    } catch (error) {
        console.error(error);
    }
}

export async function getFiles(sessionId) {
    const config = useRuntimeConfig();

    const jwtCookie = useCookie('jwt');

    try {
        const res = await $fetch(`${config.public.apiUri}/files/${sessionId}`, {
            headers: {
                Authorization: `Bearer ${jwtCookie.value}`
            },
            server: false
        });

        const results = JSON.parse(res);

        if (results.success) {
            return results.response;
        }
    } catch (error) {
        console.error(error);
    }
}

export async function deleteFile(filename, sessionId) {
    const config = useRuntimeConfig();

    const jwtCookie = useCookie('jwt');

    try {
        await $fetch(`${config.public.apiUri}/files/${sessionId}/${filename}`, {
            method: 'DELETE',
            headers: {
                Authorization: `Bearer ${jwtCookie.value}`
            },
            server: false
        });
    } catch (error) {
        console.error(error);
    }
}