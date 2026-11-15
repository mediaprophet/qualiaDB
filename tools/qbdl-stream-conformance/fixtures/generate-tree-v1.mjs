// Independent reference: Node SHA-384 and recursive tree/proof construction.
// Does not import production Rust or its encoder. Test-only deterministic bytes.
import { createHash } from 'node:crypto';
const sha = (...x) => createHash('sha384').update(Buffer.concat(x)).digest();
const be = (n, width) => { const b=Buffer.alloc(width); let v=BigInt(n); for(let i=width-1;i>=0;i--){b[i]=Number(v&255n);v>>=8n;} return b; };
const domain = (n, name) => Buffer.concat([Buffer.from([n]),Buffer.from('qbdl-stream-pq-1/'+name+'/v1'),Buffer.from([0])]);
const node = (a,b) => sha(domain(1,'node'),a,b);
const split = n => { let k=1; while(k*2<n)k*=2; return k; };
function root(xs){if(xs.length===1)return xs[0];const k=split(xs.length);return node(root(xs.slice(0,k)),root(xs.slice(k)));}
function path(xs,i){if(xs.length===1)return [];const k=split(xs.length);return i<k?[...path(xs.slice(0,k),i),root(xs.slice(k))]:[...path(xs.slice(k),i-k),root(xs.slice(0,k))];}
export function generate(){
  const cases=[];
  for(const [count,rep] of [[0,0],[1,0],[2,0],[3,0],[5,0],[17,0],[65,0],[0,1],[3,1]]){
    const length=count===0?0:(count-1)*16384+37;
    const epoch=rep?7:0;
    const context=Buffer.from(Array.from({length:48},(_,i)=>i));
    const common=Buffer.concat([context,be(19,4),be(29,8),be(rep,1),be(epoch,4),be(length,8)]);
    const leaves=[];
    for(let i=0;i<count;i++){
      const logical=i===count-1?37:16384;
      const bytes=Buffer.from(Array.from({length:logical+16*rep},(_,j)=>(i*31+j*7)&255));
      leaves.push(sha(domain(0,'leaf'),common,be(i,8),be(logical,4),be(bytes.length,4),bytes));
    }
    const digest=count?root(leaves):sha(domain(2,'empty'),common);
    cases.push({count,representation:rep,epoch,logical_length:length,root:digest.toString('hex'),proofs:leaves.map((_,i)=>path(leaves,i).map(x=>x.toString('hex')))});
  }
  return {profile:'qbdl-stream-pq-1/tree-v1',generator:'Independent Node crypto SHA384 + recursive RFC9162-shaped tree; source generate-tree-v1.mjs',context_hex:Buffer.from(Array.from({length:48},(_,i)=>i)).toString('hex'),entry:19,generation:29,payload_recipe:'byte j in encoded leaf i = (i*31+j*7) mod 256; final logical length 37',cases};
}
