import {copyFile} from 'node:fs/promises';
for(const name of ['index.html','style.css'])await copyFile(name,`dist/${name}`);
