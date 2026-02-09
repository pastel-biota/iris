new-route NAME:
  pnpm exec scaffdog generate new-route -fa name:{{ NAME }} > /dev/null

sync-postman:
  #!/usr/bin/env /usr/bin/zsh

  eval $(cat .env | grep -v "#" | sed 's/^\(.\+=.\+\)/export \1/' - )
  echo ":: Environment variable is loaded"

  echo ":: Retrieving local scheme"
  curl -s http://localhost:8080/openapi.json -o openapi.json 

  echo ":: Retrieving from Postman"
  curl "https://api.getpostman.com/collections/${POSTMAN_COLLECTION_ID}" \
    -H "X-Api-Key: ${POSTMAN_API_KEY}" \
  | jq '.collection' > postman-collection.json

  echo ":: Migrating the collection"
  pnpm openapi2postmanv2 -s openapi.json --sync postman-collection.json --sync-options syncExamples=true -o synced-collection.json

  echo ":: Updating the collection"
  curl -s -X PUT "https://api.getpostman.com/collections/${POSTMAN_COLLECTION_ID}" \
    -H "X-Api-Key: ${POSTMAN_API_KEY}" \
    -H "Content-Type: application/json" \
    -d @<(jq '{collection: .}' synced-collection.json)
  echo

  echo ":: Cleaning the files"
  rm -v openapi.json postman-collection.json synced-collection.json

  echo
  echo "[✓] All done!"

