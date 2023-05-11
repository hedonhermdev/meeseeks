import chromadb
from flask import Flask, jsonify
from flask import Flask, request, abort, jsonify
from string import Template
from uuid import uuid1

COLLECTION_NAME = "connected-agents"
DOCUMENT_TEMPLATE = Template("""
---NAME---
$name
---COMMANDS---
$commands
---EXAMPLES---
$examples
""")


class ToolDB:
    def __init__(self):
        self.client = chromadb.Client()
        self.collection = self.client.create_collection(COLLECTION_NAME)

        # self.collection.create_index()
        self.collection.add(documents=[""], ids=["none"])

    def add_tool(self, tool):
        documents = self._make_documents(tool)
        metadatas = [{'name': tool['name']} for _ in documents]
        ids = [str(uuid1()) for _ in documents]
        self.collection.add(
            documents=documents,
            metadatas=metadatas,
            ids=ids
        )

    def get_matching_tool(self, query):
        query_texts = [query,]
        n_results = 1
        
        results = self.collection.query(query_texts=query_texts, n_results=n_results, include=['metadatas'])
        print(results)
        
        return results['metadatas'][0][0]['name']
        
    def _make_documents(self, tool):
        docs = []
        docs += tool['commands']
        docs += tool['examples'].split('\n')

        return docs

app = Flask(__name__)
tooldb = ToolDB()

@app.route("/tool/add", methods=["POST"])
def add():
    try:
        print(request.json)
        tool = request.json['tool']
        tooldb.add_tool(tool)
        return jsonify({"tool": tool}), 201
    except Exception as e:
        print(e)
        return jsonify({"message": "invalid request"}), 400

@app.route("/tool/match", methods=["GET"])
def match():
    query = request.args.get("task")
    tool = tooldb.get_matching_tool(query)
    print(tool)
    if tool == "none":
        return jsonify({"message": "not found"}), 404
    return jsonify({"name": tool}), 200

if __name__ == '__main__':
    app.debug == True
    app.run(host="0.0.0.0", port=5000, threaded=False)
