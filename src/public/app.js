String.prototype.toTitleCase = function() {
    return this.charAt(0).toUpperCase() + this.substr(1);
};

String.prototype.plural = function(n) {
    return n >= 2 ? `${this}s` : this;
};

shouldLogin();
document.addEventListener('DOMContentLoaded', documentReady, false);

function documentReady() {
    class Files {
        constructor() {
            this.files = [];
            setInterval(() => {
                this.lookForExpired();
            }, 15 * 1000);
        }

        loadCache() {
            const filesData = JSON.parse(localStorage.getItem('files') || '[]');
            for (const data of filesData) {
                const file = new File(null);
                this.files.push(file);
                file.buildBase(data.name);
                file.buildDetails(data);
                document.querySelector('.files').append(file.node);
            }
            this.lookForExpired();
            document.body.classList.add('ready');
        }

        add(file) {
            document.querySelector('.files').append(file.node);
            
            this.files.push(file);
            this.updateButtons();
            this.save();
        }

        remove(file) {
            file.node.remove();
            
            const index = this.files.indexOf(file);
            if (index === -1) return;
            this.files.splice(index, 1);
            this.updateButtons();
            this.save();
        }

        clear() {
            for (const file of this.files) {
                file.node.remove();
            }
            this.files = [];
            this.updateButtons();
            this.save();
        }

        clearExpired() {
            this.files = this.files.filter(file => {
                if (file.state === 'expired') {
                    file.node.remove();
                    return false;
                }
                return true;
            });
            this.updateButtons();
            this.save();
        }

        save() {
            localStorage.setItem('files', JSON.stringify(
                this.files.filter(f => f.state === 'available' || f.state === 'expired').map(f => f.info))
            );
        }

        lookForExpired() {
            const checkRemote = [];
            const now = (new Date()).getTime() / 1000;
            for (const file of this.files.filter(f => f.state === 'available')) {
                if (file.info.expiration.date.timestamp < now) {
                    file.buildExpired(false);
                } else {
                    checkRemote.push(file);
                }
            }

            if (checkRemote.length > 0 && document.visibilityState === 'visible') {
                const req = new XMLHttpRequest();
                req.open('GET', `/valids/${checkRemote.map(f => f.info.alias.short).join('+')}`, true);
                req.responseType = 'json';
                req.onload = (event) => {
                    if (req.status === 200) {
                        for (let i = 0; i < checkRemote.length; i++) {
                            if (!req.response.valids[i]) {
                                checkRemote[i].buildExpired(true);
                            }
                        }
                    } else {
                        console.error(`An error occurred while checking expired remote files: ${req.response.error}.`);
                    }
                    this.updateButtons();
                };
                req.send();
            } else {
                this.updateButtons();
            }
        }

        updateButtons() {
            document.body.classList.toggle('has-file', this.files.length >= 1);
            document.body.classList.toggle('has-clearable', this.files.filter(f => f.state !== 'upload').length >= 1);
            document.body.classList.toggle('has-availables', this.files.filter(f => f.state === 'available').length >= 2);
            
            const aliasGroup = this.files.filter(f => f.state === 'available').map(f => f.info.alias.short).join('+');
            document.querySelector('.archive-link').setAttribute('data-clipboard-text', `${window.location.origin}/${aliasGroup}`);
        }
    }

    class File {
        constructor(fileRef) {
            this.fileRef = fileRef;
        }

        buildBase(filename) {
            this.node = document.createElement('div');
            this.node.classList.add('file');
            
            this.name = document.createElement('div');
            this.name.classList.add('name');
            this.name.innerText = filename;
            this.node.append(this.name);
        }

        startUpload() {
            this.state = 'upload';

            this.progressBar = document.createElement('div');
            this.progressBar.classList.add('progress-bar');
            this.percent = document.createElement('div');
            this.percent.classList.add('percent');
            this.node.append(this.progressBar, this.percent);

            const req = new XMLHttpRequest();
            req.open('POST', '/', true);
            req.setRequestHeader('X-Filename', encodeURIComponent(this.fileRef.name));
            req.setRequestHeader('Content-Type', this.fileRef.type);
            req.responseType = 'json';

            this.progress = 0;
            let showingProgress = false;
            const showProgressTimeout = setTimeout(() => {
                showingProgress = true;
                this.updateProgressBar();
            }, 500);
            
            req.upload.onprogress = (event) => {
                this.progress = event.loaded / event.total;
                if (showingProgress) {
                    this.updateProgressBar();
                    this.percent.innerText = `${Math.floor(this.progress * 100)}%`;
                }
            };
            req.onload = (event) => {
                clearTimeout(showProgressTimeout);
                if (req.status === 201) {
                    const resp = req.response; 
                    delete resp.success;
                    this.info = resp;

                    setTimeout(() => {
                        this.buildDetails(resp);
                        FILES.updateButtons();
                        FILES.save();
                    }, showingProgress ? 550 : 0);
                } else {
                    this.node.classList.add('error');
                    this.progressBar.style.backgroundColor = '#ff5d24';
                    this.progressBar.style.width = '100%';

                    setTimeout(() => {
                        this.buildError(req.response);
                        FILES.save();
                    }, showingProgress ? 550 : 0);
                }
            };
            req.send(this.fileRef);
        }

        updateProgressBar() {
            this.progressBar.style.backgroundColor = `rgb(${(0x15 - 0xff) * this.progress + 0xff}, ${(0xb1 - 0xc6) * this.progress + 0xc6}, ${(0x54 - 0x1d) * this.progress + 0x1d})`;
            this.progressBar.style.width = `${this.progress * 100}%`;
        }

        buildDetails(data) {
            this.state = 'available';
            this.info = data;

            const link = document.createElement('div');
            link.classList.add('link', 'selectable');
            link.innerText = this.info.link.short;

            const info = document.createElement('div');
            info.classList.add('info');

            const qrcodeWrapper = document.createElement('div');
            qrcodeWrapper.classList.add('qrcode', 'hidden');
            const qrcode = new QRCode(qrcodeWrapper, {
                text: this.info.link.short,
                width: 120,
                height: 120,
                colorDark : '#131313',
                colorLight : 'TEMPLATE_COLOR',
                correctLevel : QRCode.CorrectLevel.L
            });
            qrcodeWrapper.onclick = () => {
                qrcodeWrapper.classList.toggle('hidden');
            };

            const right = document.createElement('div');
            right.classList.add('right');

            const details = document.createElement('div');
            details.classList.add('details');

            const size = document.createElement('div');
            size.classList.add('item');
            const sizeLabel = document.createElement('div');
            sizeLabel.classList.add('label');
            sizeLabel.innerText = 'Size';
            const sizeContent = document.createElement('div');
            sizeContent.title = 'Click to toggle between formats';
            sizeContent.classList.add('content', 'clickable');
            sizeContent.innerText = this.info.size.readable;
            let sizeFormat = 'readable';
            sizeContent.addEventListener('click', () => {
                switch (sizeFormat) {
                    case 'readable':
                        sizeContent.innerText = `${this.info.size.bytes.toLocaleString()} B`;
                        sizeFormat = 'bytes';
                        break;
                    case 'bytes':
                        sizeContent.innerText = this.info.size.readable;
                        sizeFormat = 'readable';
                        break;
                }
            });

            const longAlias = document.createElement('div');
            longAlias.classList.add('item');
            const longAliasLabel = document.createElement('div');
            longAliasLabel.classList.add('label');
            longAliasLabel.innerText = 'Long alias';
            const longAliasContent = document.createElement('div');
            longAliasContent.classList.add('content', 'selectable');
            longAliasContent.innerText = this.info.alias.long;
            
            const expiration = document.createElement('div');
            expiration.classList.add('item');
            const expirationLabel = document.createElement('div');
            expirationLabel.classList.add('label', 'clickable');
            expirationLabel.innerText = 'Duration';
            const expirationContent = document.createElement('div');
            expirationContent.classList.add('content', 'clickable');
            expirationContent.innerText = this.info.expiration.duration.readable;
            let expirationFormat = 'duration';
            const updateExpirationLabel = () => {
                switch (expirationFormat) {
                    case 'date':
                        expirationLabel.innerText = 'Expiration'
                        expirationContent.innerText = this.info.expiration.date.readable;
                        break;
                    case 'duration':
                        expirationLabel.innerText = 'Duration'
                        expirationContent.innerText = this.info.expiration.duration.readable;
                        break;
                }
            }
            [expirationLabel, expirationContent].forEach((elem) => {
                elem.title = 'Click to toggle between formats';
                elem.addEventListener('click', (event) => {
                    expirationFormat = expirationFormat === 'date' ? 'duration' : 'date';
                    updateExpirationLabel();
                })
            });

            const bottom = document.createElement('div');
            bottom.classList.add('bottom');

            const actions = document.createElement('div');
            actions.classList.add('actions');

            const copyShort = document.createElement('div');
            copyShort.classList.add('copy-short', 'clickable', 'copy');
            copyShort.setAttribute('data-clipboard-text', this.info.link.short);
            copyShort.innerText = 'Copy Link';

            const dropdown = document.createElement('div');
            dropdown.classList.add('dropdown', 'clickable');

            const menu = document.createElement('div');
            menu.classList.add('menu');

            const download = document.createElement('div');
            download.classList.add('item');
            download.innerText = 'Download';
            download.addEventListener('click', () => {
                document.location = this.info.link.short;
            });

            const separator = document.createElement('div');
            separator.classList.add('separator');

            const copyLong = document.createElement('div');
            copyLong.classList.add('item', 'copy');
            copyLong.setAttribute('data-clipboard-text', this.info.link.long);
            copyLong.innerText = 'Copy long link';

            const newAlias = document.createElement('div');
            newAlias.classList.add('item', 'sub-menu');
            newAlias.innerText = 'Generate new alias';

            const aliasMenu = document.createElement('div');
            aliasMenu.classList.add('menu');

            for (const t of ['short', 'long', 'both']) {
                const type = document.createElement('div');
                type.classList.add('item');
                type.innerText = t.toTitleCase();
                type.addEventListener('click', () => {
                    if (confirm(`Generating ${t === 'both' ? 'new aliases' : 'a new alias'} will make all people with a current link unable to access it. Confirm?`)) {
                        const req = new XMLHttpRequest();
                        let path = `/${this.info.alias.short}/aliases`;
                        if (t !== 'both') {
                            path = path.concat(`/${t}`);
                        }
                        req.open('PATCH', path, true);
                        req.setRequestHeader('Authorization', this.info.admin);
                        req.setRequestHeader('X-Authorization', this.info.admin);
                        req.responseType = 'json';
                        req.onload = (event) => {
                            if (req.status === 200) {
                                if (t === 'short' || t === 'both') {
                                    this.info.alias.short = req.response.alias.short;
                                    this.info.link.short = req.response.link.short;
                                    link.innerText = this.info.link.short;
                                    copyShort.setAttribute('data-clipboard-text', this.info.link.short);
                                    qrcode.makeCode(this.info.link.short);
                                }
                                if (t === 'long' || t === 'both') {
                                    this.info.alias.long = req.response.alias.long;
                                    this.info.link.long = req.response.link.long;
                                    longAliasContent.innerText = this.info.alias.long;
                                    copyLong.setAttribute('data-clipboard-text', this.info.link.long);
                                }
                                FILES.save();
                            } else {
                                alert(`An error occured while trying to generate ${t === 'both' ? 'new aliases' : 'a new alias'}: ${req.response.error}.`);
                            }
                        }
                        req.send();
                    };
                });
                aliasMenu.append(type);
            }

            const extend = document.createElement('div');
            extend.classList.add('item');
            extend.innerText = 'Extend duration';
            extend.addEventListener('click', () => {
                if (confirm('Extending this file will try to reset its duration to its initial one, which will still count toward your quota. Confirm?')) {
                    const req = new XMLHttpRequest();
                    req.open('PATCH', `/${this.info.alias.short}/expiration`, true);
                    req.setRequestHeader('Authorization', this.info.admin);
                    req.setRequestHeader('X-Authorization', this.info.admin);
                    req.responseType = 'json';
                    req.onload = (event) => {
                        if (req.status === 200) {
                            delete req.response.success;
                            this.info.expiration = req.response;
                            FILES.save();
                            updateExpirationLabel();
                        } else {
                            alert(`An error occured while trying to extend expiration: ${req.response.error}.`);
                        }
                    };
                    req.send();
                }
            });

            const downloads = document.createElement('div');
            downloads.classList.add('item', 'sub-menu');
            downloads.innerText = 'Limit downloads';

            const downloadsMenu = document.createElement('div');
            downloadsMenu.classList.add('menu');

            for (const n of [1, 3, 5, 10, 25, 100, 0]) {
                const count = document.createElement('div');
                count.classList.add('item');
                count.innerText = n ? `${n} ${'download'.plural(n)}` : 'Unlimited';
                count.addEventListener('click', () => {
                    const req = new XMLHttpRequest();
                    req.open('PATCH', `/${this.info.alias.short}/downloads/${n}`, true);
                    req.setRequestHeader('Authorization', this.info.admin);
                    req.setRequestHeader('X-Authorization', this.info.admin);
                    req.responseType = 'json';
                    req.onload = (event) => {
                        if (req.status === 200) {
                        } else {
                            alert(`An error occurred while trying to set downloads limit: ${req.response.error}.`);
                        }
                    };
                    req.send();
                });
                downloadsMenu.append(count);
            }

            const forget = document.createElement('div');
            forget.classList.add('item', 'warning');
            forget.innerText = 'Forget';
            forget.addEventListener('click', (event) => {
                if (confirm('Forgetting this file will still count toward your quota. Confirm?')) {
                    FILES.remove(this);
                }
            });

            const revoke = document.createElement('div');
            revoke.classList.add('item', 'destructive');
            revoke.innerText = 'Revoke';
            revoke.addEventListener('click', () => {
                if (confirm('Revoking this file will make all people with a link unable to access it. Confirm?')) {
                    const req = new XMLHttpRequest();
                    req.open('DELETE', `/${this.info.alias.short}`, true);
                    req.setRequestHeader('Authorization', this.info.admin);
                    req.setRequestHeader('X-Authorization', this.info.admin);
                    req.responseType = 'json';
                    req.onload = (event) => {
                        if (req.status === 200) {
                            FILES.remove(this);
                        } else {
                            alert(`An error occurred while trying to revoke this file: ${req.response.error}.`);
                        }
                    };
                    req.send();
                }
            });

            info.append(qrcodeWrapper, right);
            right.append(details, bottom);
            details.append(size, longAlias, expiration);
            size.append(sizeLabel, sizeContent);
            longAlias.append(longAliasLabel, longAliasContent);
            expiration.append(expirationLabel, expirationContent);
            bottom.append(actions);
            actions.append(copyShort, dropdown, menu);
            newAlias.append(aliasMenu);
            downloads.append(downloadsMenu);
            menu.append(download, separator.cloneNode(), copyLong, newAlias, separator, extend, downloads, separator.cloneNode(), forget, revoke);

            if (this.progressBar) this.progressBar.remove();
            if (this.percent) this.percent.remove();
            this.node.append(link, info);
        }

        buildError(data) {
            this.state = 'error';

            const remove = document.createElement('div');
            remove.classList.add('remove', 'clickable');
            remove.addEventListener('click', () => {
                FILES.remove(this);
            })

            const error = document.createElement('div');
            error.classList.add('error-message');
            error.innerText = data.error.toTitleCase();

            if (this.progressBar) this.progressBar.remove();
            if (this.percent) this.percent.remove();
            this.node.append(remove, error);
        }

        buildExpired(remotely) {
            this.state = 'expired';
            this.node.classList.add('expired');

            const expired = document.createElement('div');
            expired.classList.add('expired');
            expired.addEventListener('click', (event) => {
                FILES.remove(this);
            });

            const label = document.createElement('div');
            label.classList.add('label');
            label.innerText = remotely ? 'Drained' : 'Expired';

            expired.append(label);
            this.node.append(expired);
        }
    }

    function uploadFiles(files) {
        for (const f of files) {
            if (!f.name) continue;
            const file = new File(f);
            file.buildBase(f.name);
            file.startUpload();
            FILES.add(file);
        }
    }

    // Form input.
    document.querySelectorAll('input[type=file]').forEach((elem) => {
        elem.addEventListener('change', (event) => {
            uploadFiles(event.target.files);
        });
    });

    // Drag and drop.
    ['drag', 'dragstart', 'dragend', 'dragover', 'dragenter', 'dragleave', 'drop'].forEach((name) => {
        document.body.addEventListener(name, event => {
            event.preventDefault();
            event.stopPropagation();
        })
    });
    ['dragover', 'dragenter'].forEach((name) => {
        document.body.addEventListener(name, event => {
            document.body.classList.add('dragging');
        })
    });
    ['dragleave', 'dragend', 'drop'].forEach((name) => {
        document.body.addEventListener(name, event => {
            document.body.classList.remove('dragging');
        })
    });
    document.body.addEventListener('drop', (event) => {
        const files = [];
        for (let item of event.dataTransfer.items) {
            if (item.kind !== 'file') continue;
            if (item.webkitGetAsEntry && item.webkitGetAsEntry().isDirectory) continue;
            files.push(item.getAsFile());
        }
        uploadFiles(files);
    });

    document.querySelector('.clear > .session').addEventListener('click', (event) => {
        if (confirm('You are about to clear all your files, but they will still count toward your quota. Confirm?')) {
            FILES.clear();
        }
    });

    document.querySelector('.clear > .expired').addEventListener('click', (event) => {
        FILES.clearExpired();
    });

    document.addEventListener('click', (event) => {
        if (event.target.nodeType === Node.ELEMENT_NODE && event.target.classList.contains('dropdown')) {
            for (const opened of document.querySelectorAll('.menu.opened')) {
                opened.classList.remove('opened');
            }
            if (!event.target.nextSibling.classList.contains('opened')) {
                event.target.nextSibling.classList.add('opened');
            }
        } else if (event.target.nodeType === Node.ELEMENT_NODE && event.target.classList.contains('sub-menu')) {
            for (const opened of document.querySelectorAll('.sub-menu > .menu.opened')) {
                opened.classList.remove('opened');
            }
            event.target.lastChild.classList.add('opened');
        } else {
            for (const opened of document.querySelectorAll('.menu.opened')) {
                opened.classList.remove('opened');
            }
        }
    });

    document.querySelector('.files').addEventListener('mouseover', (event) => {
        for (const opened of document.querySelectorAll('.sub-menu > .menu.opened')) {
            opened.classList.remove('opened');
        }
    });

    const FILES = new Files();
    FILES.loadCache();

    new ClipboardJS('.copy');
}

function shouldLogin() {
    const req = new XMLHttpRequest();
    req.open('GET', '/auth', true);
    req.responseType = 'json';
    req.onload = (_event) => {
        if (req.status === 200) {
            if (req.response.required) {
                window.location = '/login/';
            }
        } else {
            console.error(`An error occurred while checking for session token: ${req.response.error}.`);
        }
    };
    req.send();
}