const fs = require("fs");
const assert = require("assert");

const NS_PER_SEC = 1e9;
const ITER = 10000;

const content = fs.readFileSync("./foo.json");
const earlier = process.hrtime();

for (let i = 0; i < ITER; i++) {
  let _json = JSON.parse(content);
//   assert(_json["web-app"]["servlet"][0]["servlet-name"] === "cofaxCDS");
}

const diff = process.hrtime(earlier);
// [ 1, 552 ]
const ns = diff[0] * NS_PER_SEC + diff[1];
console.log(
  `JSON.parse took ${ns} nanoseconds => ${(ns / NS_PER_SEC) * 1000}ms`
);
