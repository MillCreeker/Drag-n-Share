import { generateKeyPair, deriveSharedSecret, convertKeyToBase64, importKeyFromBase64, encryptData, decryptData, downloadDataUrl } from '~/public/utils/utils';

export function trnsRegister(socket) {
    const jwtCookie = useCookie('jwt');

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'register',
        data: JSON.stringify({})
    }));
}

export async function trnsRequestFile(socket, filename) {
    const jwtCookie = useCookie('jwt');

    const keyPair = await generateKeyPair();
    const publicKey = keyPair.publicKey;
    const privateKey = keyPair.privateKey;

    const publicKeyCookie = useCookie(`${filename}-publicKey`);
    const privateKeyCookie = useCookie(`${filename}-privateKey`);

    publicKeyCookie.value = publicKey;
    privateKeyCookie.value = privateKey;

    const base64Key = await convertKeyToBase64(publicKey);
    console.log(base64Key);
    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'request-file',
        data: JSON.stringify({
            public_key: base64Key,
            filename: filename
        })
    }));
}

export async function trnsWsHandleMessage(socket, message) {
    console.log('Received:', message);
    const requestId = message.request_id;
    const data = message.data;

    switch (message.command) {
        case 'acknowledge-file-request':
            await trnsHandleAcknowledgeFileRequest(socket, requestId, data);
            break;
        case 'prepare-for-file-transfer':
            await trnsHandlePrepareForFileTransfer(socket, requestId, data);
            break;
        case 'send-next-chunk':
            await trnsHandleSendNextChunk(socket, requestId, data);
            break;
        case 'add-chunk':
            await trnsHandleAddChunk(socket, requestId, data);
            break;
        default:
            console.error('Unknown command:', message.command);
            break;
    }
}

async function trnsHandleAcknowledgeFileRequest(socket, requestId, data) {
    const clientPublicKey = await importKeyFromBase64(data.public_key);
    const filename = data.filename;
    const userId = data.user_id;

    const keyPair = await generateKeyPair();
    const publicKey = keyPair.publicKey;
    const privateKey = keyPair.privateKey;

    const secret = await deriveSharedSecret(clientPublicKey, privateKey);

    publicKeyCookie = useCookie(`${requestId}-publicKey`);
    privateKeyCookie = useCookie(`${requestId}-privateKey`);
    secretCookie = useCookie(`${requestId}-secret`);

    publicKeyCookie.value = publicKey;
    privateKeyCookie.value = privateKey;
    secretCookie.value = secret;

    const base64Key = await convertKeyToBase64(publicKey);

    const fileCookie = userCookie(`file-${filename}`);
    const file = fileCookie.value;
    const amountOfChunks = Math.ceil(file.length / 1024)

    await trnsAcknwoledgeFileRequest(socket, base64Key, amountOfChunks, filename, userId);
}

async function trnsHandlePrepareForFileTransfer(socket, requestId, data) {
    const clientPublicKey = await importKeyFromBase64(data.public_key);
    const filename = data.filename;
    const amountOfChunks = data.amount_of_chunks;

    const publicKeyCookie = useCookie(`${filename}-publicKey`);
    const privateKeyCookie = useCookie(`${filename}-privateKey`);

    const newPublicKeyCookie = useCookie(`${requestId}-publicKey`);
    const newPrivateKeyCookie = useCookie(`${requestId}-privateKey`);

    newPublicKeyCookie.value = publicKeyCookie.value;
    newPrivateKeyCookie.value = privateKeyCookie.value;

    publicKeyCookie.value = null;
    privateKeyCookie.value = null;

    const secret = await decryptData(clientPublicKey, privateKeyCookie.value);
    const secretCookie = useCookie(`${requestId}-secret`);
    secretCookie.value = secret;

    const chunkAmountCookie = useCookie(`${requestId}-chunkAmount`);
    chunkAmountCookie.value = amountOfChunks;

    await trnsReadyForFileTransfer(socket, requestId);
}

async function trnsHandleSendNextChunk(socket, requestId, data) {
    const lastChunkNr = data.last_chunk_nr;

    const fileCookie = userCookie(`file-${filename}`);
    const file = fileCookie.value;

    const cutOff = lastChunkNr * 1024 + 1024;
    const chunk = file.slice(lastChunkNr * 1024, cutOff);
    const isLastChunk = file.length <= cutOff;

    const secretCookie = useCookie(`${requestId}-secret`);
    const secret = secretCookie.value;

    const iv = await generateIv();

    const encryptedChunk = await encryptData(secret, iv, chunk);

    await trnsAddChunk(socket, requestId, isLastChunk, lastChunkNr + 1, encryptedChunk, iv);

    console.log(filename, lastChunkNr + 1);
}

async function trnsHandleAddChunk(socket, requestId, data) {
    const isLastChunk = data.is_last_chunk;
    const chunkNr = data.chunk_nr;
    const encryptedChunk = data.chunk;
    const iv = data.iv;

    const secretCookie = useCookie(`${requestId}-secret`);
    const secret = secretCookie.value;

    const chunk = await decryptData(secret, iv, encryptedChunk);

    const fileCookie = userCookie(`${requestId}-file`);
    let file = fileCookie.value;
    file = [file.slice(0, chunkNr * 1024), chunk, file.slice(chunkNr * 1024)].join('')
    fileCookie.value = file;

    if (isLastChunk) {
        console.log(file);
        downloadDataUrl(fileCookie.value, 'test.sh');
    }

    console.log(requestId, chunkNr);
}



async function trnsAcknwoledgeFileRequest(socket, publicKey, amountOfChunks, filename, userId) {
    const jwtCookie = useCookie('jwt');

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'acknowledge-file-request',
        data: JSON.stringify({
            public_key: publicKey,
            amount_of_chunks: amountOfChunks,
            filename: filename,
            user_id: userId
        })
    }));
}

async function trnsReadyForFileTransfer(socket, requestId) {
    const jwtCookie = useCookie('jwt');

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'ready-for-file-transfer',
        data: JSON.stringify({
            request_id: requestId
        })
    }));
}

async function trnsAddChunk(socket, requestId, isLastChunk, chunkNr, encryptedChunk, iv) {
    const jwtCookie = useCookie('jwt');

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'add-chunk',
        data: JSON.stringify({
            request_id: requestId,
            is_last_chunk: isLastChunk,
            chunk_nr: chunkNr,
            chunk: encryptedChunk,
            iv: iv
        })
    }));
}