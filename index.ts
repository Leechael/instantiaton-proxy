import fs from 'node:fs'
import { getClient, getContract, signCertificate, type SerMessageMessageOutput, randomHex } from '@phala/sdk'
import { Keyring } from '@polkadot/api'
import { Abi } from '@polkadot/api-contract'
import { u8aToHex, hexToU8a, compactAddLength } from '@polkadot/util'


async function main() {
  const client = await getClient({ transport: 'wss://poc6.phala.network/ws' })
  const contract = await getContract({
    client,
    contractId: '0xf25ebc68e99e095e6a3e991b313a6c2fcc952d7c2625f1a263169ef36f5fe517',
    abi: fs.readFileSync('./artifacts/instantiate_proxy.json', 'utf8'),
  })

  const constructorName = 'create'
  const args = ['PhalaWorld', 'Become a survivor in the IMMERSIVE WORLD']

  //
  //
  //
  const abi = new Abi(fs.readFileSync('./artifacts/phalaworld.json', 'utf8'))
  const selector = abi.constructors.find(i => i.identifier === constructorName)?.selector.toU8a()
  const arg0 = client.api.createType('String', args[0]).toU8a()
  const arg1 = client.api.createType('Option<String>', args[1]).toU8a()
  const encoded = compactAddLength(new Uint8Array([...arg0, ...arg1]))
  const codeHash = abi.info.source.wasmHash
  console.log('contructor & args:', constructorName, args)
  console.log('constructor selector:', u8aToHex(selector))
  console.log('arguments encoded:', u8aToHex(encoded))

  // // console.log(contract.address.toHex())
  //
  //
  console.log('Now instantiate via proxy')

  const keyring = new Keyring({ type: 'sr25519' })
  const pair = keyring.addFromUri('//Alice')
  const cert = await signCertificate({ pair })
  const result = await contract.send.instantiate(
    { address: pair.address, cert, pair },
    codeHash,
    selector,
    encoded,
    randomHex(32),
  )
  await result.waitFinalized()

  const { records } = await client.loggerContract!.tail(10, { type: 'MessageOutput', contract: contract.address.toHex(), nonce: result.nonce })
  const instantiateResult = (records?.[0] as SerMessageMessageOutput).output.result
  const raw = 'ok' in instantiateResult ? instantiateResult.ok.data : ''
  const decoded = client.api.createType('Result<Result<AccountId, Vec<u8>>, u8>', hexToU8a(raw))
  const contractId = decoded.asOk.asOk.toHex()

  console.log('The instantiated contractId: ', contractId)
}

main().catch(console.error).finally(() => process.exit())