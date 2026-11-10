#!/usr/bin/env node
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import puppeteer from 'puppeteer';

const here = new URL('.', import.meta.url);
const browser = await puppeteer.launch(JSON.parse(await readFile(new URL('puppeteer.json', here))));
try {
  const page = await browser.newPage();
  await page.setRequestInterception(true);
  page.on('request', request => request.abort());
  await page.addScriptTag({path: fileURLToPath(new URL('node_modules/dompurify/dist/purify.js', here))});
  const output = await page.evaluate(() => {
    const dirty = '<svg xmlns="http://www.w3.org/2000/svg"><a><animate attributeName="href" values="x;javascript:alert(1)" dur="0.01s" fill="freeze"/></a></svg>';
    const config = {PARSER_MEDIA_TYPE: 'application/xhtml+xml', ALLOWED_TAGS: ['svg', 'a', 'animate'], ALLOWED_ATTR: ['attributeName', 'values', 'dur', 'fill']};
    return {xhtml: DOMPurify.sanitize(dirty, config), html: DOMPurify.sanitize(dirty)};
  });
  assert(!output.xhtml.includes('attributeName="href"'), 'XHTML sanitizer retained the SMIL href rewrite attribute');
  assert(!/attributename="href"/i.test(output.html), 'default sanitizer retained the href rewrite attribute');
  console.log('PASS sanitizer removes href rewrite attributes without executing or inserting the untrusted result');
} finally {
  await browser.close();
}
