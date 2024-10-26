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

function arrayBufferToBase64(arrayBuffer) {
    let binary = '';
    const bytes = new Uint8Array(arrayBuffer);
    const len = bytes.byteLength;
    for (let i = 0; i < len; i++) {
        binary += String.fromCharCode(bytes[i]);
    }
    return btoa(binary);
}

function base64ToArrayBuffer(base64) {
    const binary = atob(base64);
    const len = binary.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
        bytes[i] = binary.charCodeAt(i);
    }
    return bytes.buffer;
}

export async function convertKeyToBase64(key) {
    const rawKey = await crypto.subtle.exportKey('raw', key);
    const base64Key = arrayBufferToBase64(rawKey);

    return base64Key;
}

export async function importKeyFromBase64(base64Key) {
    const rawKey = base64ToArrayBuffer(base64Key);

    const key = await crypto.subtle.importKey(
        'raw',
        rawKey,
        {
            name: "ECDH",
            namedCurve: "P-256",
        },
        true, // extractable
        [] 
    );

    return key;
}

export async function exportPrivateKeyToBase64(privateKey) {
    const rawKey = await crypto.subtle.exportKey('pkcs8', privateKey);
    const base64Key = arrayBufferToBase64(rawKey);
    return base64Key;
}

export async function importPrivateKeyFromBase64(base64Key) {
    const rawKey = base64ToArrayBuffer(base64Key);
    const privateKey = await crypto.subtle.importKey(
        'pkcs8',
        rawKey,
        {
            name: "ECDH",
            namedCurve: "P-256",
        },
        true, // extractable
        ["deriveKey", "deriveBits"]
    );
    return privateKey;
}

export async function exportSharedSecretToBase64(sharedSecret) {
    const rawKey = await crypto.subtle.exportKey('raw', sharedSecret);
    const base64Key = arrayBufferToBase64(rawKey);
    return base64Key;
}

export async function importSharedSecretFromBase64(base64Key) {
    const rawKey = base64ToArrayBuffer(base64Key);
    const sharedSecret = await crypto.subtle.importKey(
        'raw',
        rawKey,
        {
            name: "AES-GCM",
            length: 256,
        },
        true, // extractable
        ["encrypt", "decrypt"]
    );
    return sharedSecret;
}

export function ivToBase64(iv) {
    return arrayBufferToBase64(iv.buffer);
}

export function base64ToIv(base64) {
    const arrayBuffer = base64ToArrayBuffer(base64);
    return new Uint8Array(arrayBuffer);
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

export function downloadDataUrl(dataUrl, filename) {
    const link = document.createElement("a");
    link.download = filename;
    link.href = dataUrl;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    // delete link;
};