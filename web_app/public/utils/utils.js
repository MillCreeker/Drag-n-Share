export async function convertFiles(files) {
    const fileArr = [];

    const toBase64 = file => new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.readAsDataURL(file);
        reader.onload = () => resolve(reader.result);
        reader.onerror = error => reject(error);
    });
    try {
        for (let i = 0; i < files.length; i++) {
            const file = files[i];

            const base64 = await toBase64(file);
            const fileString = `name:${file.name};${base64}`;

            fileArr.push({
                name: file.name,
                size: file.size,
                data: fileString
            });
        }
    } catch (e) {
        console.error(e);
    }

    return fileArr;
}

export async function generateKeyPair() {
    const keyPair = await crypto.subtle.generateKey(
        {
            name: "ECDH",
            namedCurve: "P-256",
        },
        true, // extractable
        ["deriveKey", "deriveBits"]
    );

    return keyPair;
}

export async function deriveSharedSecret(privateKey, publicKey) {
    const sharedSecret = await crypto.subtle.deriveKey(
        {
            name: "ECDH",
            public: publicKey,
        },
        privateKey,
        {
            name: "AES-GCM",
            length: 256,
        },
        true, // extractable
        ["encrypt", "decrypt"]
    );

    return sharedSecret;
}

export async function generateIv() {
    const iv = new Uint8Array(12);
    crypto.getRandomValues(iv);

    return iv;
}

export async function encryptData(sharedSecret, iv, data) {
    const encodedData = new TextEncoder().encode(data);

    const encryptedData = await crypto.subtle.encrypt(
        {
            name: "AES-GCM",
            iv: iv,
        },
        sharedSecret,
        encodedData
    );

    return encryptedData;
}

export async function decryptData(sharedSecret, iv, encryptedData) {
    const decryptedData = await crypto.subtle.decrypt(
        {
            name: "AES-GCM",
            iv: iv,
        },
        sharedSecret,
        encryptedData
    );

    return new TextDecoder().decode(decryptedData);
}