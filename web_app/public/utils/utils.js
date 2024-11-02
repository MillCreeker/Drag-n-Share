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

export async function generateIv() {
    const iv = new Uint8Array(12);
    crypto.getRandomValues(iv);

    return iv;
}

export function ivToBase64(iv) {
    return arrayBufferToBase64(iv.buffer);
}

export function base64ToIv(base64) {
    const arrayBuffer = base64ToArrayBuffer(base64);
    return new Uint8Array(arrayBuffer);
}

export function arrayBufferToHex(arrayBuffer) {
    const byteArray = new Uint8Array(arrayBuffer);
    let hexString = '';
    byteArray.forEach(byte => {
        hexString += byte.toString(16).padStart(2, '0');
    });
    return hexString;
}

export function hexToArrayBuffer(hexString) {
    const length = hexString.length / 2;
    const byteArray = new Uint8Array(length);
    for (let i = 0; i < length; i++) {
        byteArray[i] = parseInt(hexString.substr(i * 2, 2), 16);
    }
    return byteArray.buffer;
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
    // Split the data URL to get the MIME type and the base64 data
    const [metadata, base64Data] = dataUrl.split(',');
    const mimeType = metadata.match(/:(.*?);/)[1];

    // Decode the base64 data and create a Blob
    let binary;
    try {
        binary = atob(base64Data);
    } catch (error) {
        console.error('Failed to decode base64 data:', base64Data);
        return;
    }

    const array = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
        array[i] = binary.charCodeAt(i);
    }
    const blob = new Blob([array], { type: mimeType });

    // Create an object URL for the Blob and trigger download
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();

    // Clean up
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
};

function openDatabase() {
    return new Promise((resolve, reject) => {
        const request = window.indexedDB.open('fileStorage', 1);

        request.onerror = (event) => {
            reject(event.target.error);
        };

        request.onsuccess = (event) => {
            resolve(event.target.result);
        };

        request.onupgradeneeded = (event) => {
            const db = event.target.result;
            db.createObjectStore('files', { keyPath: 'id' });
        };
    });
}

export async function storeFile(id, base64File) {
    const db = await openDatabase(); const transaction = db.transaction(['files'], 'readwrite'); const store = transaction.objectStore('files');

    // const fileBlob = new Blob([Uint8Array.from(atob(base64File), c => c.charCodeAt(0))]);
    const fileBlob = new Blob([Uint8Array.from(base64File, c => c.charCodeAt(0))]);
    const request = store.put({ id, fileBlob });

    request.onsuccess = () => {
        // console.log(`File ${id} stored successfully!`);
    };

    request.onerror = (event) => {
        console.error(`Failed to store file ${id}: `, event.target.error);
    };
}

export async function getFile(id) {
    const db = await openDatabase();
    const transaction = db.transaction(['files'], 'readonly');
    const store = transaction.objectStore('files');

    return new Promise((resolve, reject) => {
        const request = store.get(id);

        request.onsuccess = () => {
            if (request.result) {
                const fileBlob = request.result.fileBlob;
                // convert the Blob back to a Base64 string
                const reader = new FileReader();
                reader.onloadend = () => {
                    // const base64String = reader.result.split(',')[1]; // remove data URL prefix
                    resolve(reader.result);
                };
                reader.onerror = () => {
                    reject('Failed to convert Blob to Base64');
                };
                reader.readAsDataURL(fileBlob);
            } else {
                reject(`File with ID ${id} not found`);
            }
        };

        request.onerror = (event) => {
            reject(`Failed to retrieve file ${id}: ${event.target.error}`);
        };
    });
}

export async function storeLargeString(id, largeString) {
    const db = await openDatabase();
    const transaction = db.transaction(['files'], 'readwrite');
    const store = transaction.objectStore('files');

    const request = store.put({ id, content: largeString });

    request.onsuccess = () => {
        // console.log(`String ${id} stored successfully!`);
    };

    request.onerror = (event) => {
        console.error(`Failed to store string ${id}: `, event.target.error);
    };
}

export async function getLargeString(id) {
    const db = await openDatabase();
    const transaction = db.transaction(['files'], 'readonly');
    const store = transaction.objectStore('files');

    return new Promise((resolve, reject) => {
        const request = store.get(id);

        request.onsuccess = () => {
            if (request.result) {
                resolve(request.result.content);
            } else {
                reject(`String with ID ${id} not found`);
            }
        };

        request.onerror = (event) => {
            reject(`Failed to retrieve string ${id}: ${event.target.error}`);
        };
    });
}

async function clearIndexedDB() {
    const databases = await indexedDB.databases();
    databases.forEach((db) => {
        indexedDB.deleteDatabase(db.name);
    });
}

function clearCookies() {
    document.cookie.split(";").forEach((cookie) => {
        const eqPos = cookie.indexOf("=");
        const name = eqPos > -1 ? cookie.substr(0, eqPos) : cookie;
        document.cookie = `${name}=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/`;
    });
}

export async function clearStorage() {
    await clearIndexedDB();
    clearCookies();
}