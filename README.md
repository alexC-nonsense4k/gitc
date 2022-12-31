# gitc 简易版git
## 1.项目结构
```
src  
|  
+- gitMethods  
|  |  
|  + gitMethods.rs   #contain git methods such as, add, commit, merge...  
|  + mod.rs  
|  
+- gitUtils  
|  |  
|  + gitUtils.rs     #contain git utils such as, blob, commit, tree...  
|  + mod.rs  
+- main.rs           #contain function main and test functions.  
```
## 2.git基础
### 数据模型
git 将某个顶级目录中的文件和文件夹集合的历史建模为一系列快照。在 git 术语中，文件称为“blob”，它只是一堆字节。目录称为“tree”，它将名称映射到 blob 或tree(因此目录可以包含其他目录)。      
```
// a file is a bunch of bytes
type blob = array<byte>

// a directory contains named files and directories
type tree = map<string, tree | blob>

// a commit has parents, metadata, and the top-level tree
type commit = struct {
    parents: array<commit>
    author: string
    message: string
    snapshot: tree
}
```
### 对象和内容寻址
“对象”是一个 blob、tree或commit。           
```
type object = blob | tree | commit
```
在 git 数据存储中，所有对象都通过其SHA-1 散列进行内容寻址。         
```
objects = map<string, object>

def store(object):
    id = sha1(object)
    objects[id] = object

def load(id):
    return objects[id]
```
### 参考
现在，所有快照都可以通过它们的 SHA-1 哈希值来识别。这很不方便，因为人类不擅长记住 40 个十六进制字符的字符串。
git 对这个问题的解决方案是为 SHA-1 哈希提供人类可读的名称，称为“引用”。引用是指向提交的指针。与不可变的对象不同，引用是可变的（可以更新以指向新的提交）。例如，master引用通常指向开发主分支中的最新提交。          
```
references = map<string, string>

def update_reference(name, id):
    references[name] = id

def read_reference(name):
    return references[name]

def load_reference(name_or_id):
    if name_or_id in references:
        return load(references[name_or_id])
    else:
        return load(name_or_id)
```
## 3.gitUtils结构体
### blob
blob相当于文件系统中的文件，属于git中的一个基本object。     
```
pub struct blob
{
    pub name:String,
    pub contents:Vec<u8>,
    pub t:objecttype,
}
```
1.name为blob的具体名称，如"./hello.txt"   
2.contents为blob文件内容被序列化后的形式   
3.t通过枚举类型objecttype标记blob的object类型    
### tree
tree相当于文件系统中的文件夹，属于git中的一个基本object      
```
pub struct tree
{
    pub name:String,
    pub trees:BTreeMap<String,Rc<RefCell<tree>>>,
    pub blobs:BTreeMap<String,Rc<RefCell<blob>>>,
    pub t:objecttype,
}
```
1.name为tree的具体名称，如"./demo"   
2.trees记录tree这一级目录下所包含的所有tree对象   
3.blobs记录tree这一级目录下所包含的所有blob对象
4.t通过枚举类型objecttype标记tree的object类型 
### commit
commit为一次git的提交结果，属于git中的一个基本object           
```
pub struct Commit
{
    pub parents:Vec<Option<Rc<RefCell<Commit>>>>,
    pub mergeparents:Vec<Option<Rc<RefCell<Commit>>>>,
    pub author:String ,
    pub message:String,
    pub snapshot:Rc<RefCell<tree>>,
    pub t:objecttype,
}
```
1.parents为一个commit在当前分支中的所有父commit的一个vector集合      
2.mergeparents为一个commit在被merged分支中的所有父commit的一个vector集合   
3.author记录本次commit提交者的名字    
4.message记录本次提交所附加的信息   
5.snapshot为本次commit的git本地仓库的根目录的tree对象    
6.t通过枚举类型objecttype标记commit的object类型     
### branch
branch为git中的分支指针，一个branch即代表一个具体的分支，branch指向的commit为当前分支中最新提交的commit         
```
pub struct Branch
{
    pub name:String,
    pub commitpointer:Option<Rc<RefCell<Commit>>>,
    pub references:Rc<RefCell<References>>,
}
```
1.name是branch的名字      
2.commitpointer为一指针指向branch中最新的提交的那次commit    
3.references为一个<string,string> map,key为一个具体的名字字符串，value为一串SHA1码，便于从objects的map中读出具体的object。由于不同的分支上进行add操作会导致references污染，因此每个branch应当自己持有一个references    
### head
head用于管理和切换git内branch         
```
pub struct HEAD
{
    pub currentBranchName:String,
    pub branch:BTreeMap<String,Rc<RefCell<Branch>>>,
}
```
1.currentBranchName记录目前所在的branch的名字      
2.branch以map的形式记录git中所存在的所用分支，利用currentBranchName和branch结合，可以找到当前的分支    
### objects
objects用以通过SHA1码来查找对应的object         
```
pub struct Objects
{
    pub treemap:BTreeMap<String,Rc<RefCell<tree>>>,
    pub commitmap:BTreeMap<String,Rc<RefCell<Commit>>>,
    pub blobmap:BTreeMap<String,Rc<RefCell<blob>>>,
}
```
1.treemap以SHA1码和tree作为k和v进行记录         
2.commitmap以SHA1码和commit作为k和v进行记录         
3.blobmap以SHA1码和blob作为k和v进行记录      
### HashMap & BTreeMap
本次在设计底层的数据结构时，涉及Map的地方都选用了BTreeMap而不是HashMap。这是因为HashMap是一个内部key无序的Map，而BTreeMap是内部key有序的。因为git的具体方法中总是存在对object进行序列化进行SHA1计算的过程，此时如果我们选用HashMap作为基础结构，就会出现两个内部元素完全一致的Map，计算出的SHA1是完全不同，而这种不同正是因为这两个Map内元素的排列顺序不同导致的，采用BTreeMap则会避免这种错误。     
## 4.gitMethods git方法
### gitAdd
```
pub fn gitAdd(path:String,objects:&mut Objects,head:&mut HEAD,persistence:bool)
{
    let mut reference= head.branch.get(&head.currentBranchName).cloned().unwrap().borrow_mut().references.clone();
    let p=Path::new(&(path));
    if p.is_file()
    {
        let mut file=blob::new(path.clone());
        file.getContents(path.clone());
        let file_rc=Rc::new(RefCell::new(file));
        let SHA1id=getSHA1(&serialize(&file_rc).unwrap());
        if !objects.blobmap.contains_key(&SHA1id)
        {
            objects.blobmap.insert(SHA1id.clone(),file_rc.clone());
            reference.borrow_mut().refermap.insert(path.clone(),SHA1id.clone());
            let part1=&SHA1id[0..2];
            let part2=&SHA1id[2..40];
            let mut dir_path="./.gitc/objects/".to_string();
            let path1=dir_path+&String::from(part1);
            let dir=Path::new(&path1);
            if dir.exists()==false
            {
                fs::create_dir(&path1);
            }
            let path_temp=path1+&String::from("/");
            let path2=path_temp+&String::from(part2);
            let mut f=File::create(&path2);
            f.unwrap().write(&serialize(&file_rc).unwrap());
            let mut fathername=fatherName(&(path));
            let mut sonname=path.clone();
            let mut sonpath=p.clone();
            let mut sonobj=Rc::new(RefCell::new(tree::new(String::from(""))));
            while !fathername.eq("")
            {
                let mut tree_temp_rc=Rc::new(RefCell::new(tree::new(fathername.clone())));

                if reference.borrow_mut().refermap.contains_key(&fathername)
                {
                    let mut temp=load_reference_tree(&reference,objects,fathername.clone());
                    for (key,value) in temp.borrow().blobs.clone()
                    {
                        tree_temp_rc.borrow_mut().blobs.insert(key,value);
                    }
                    for (key,value) in temp.borrow().trees.clone()
                    {
                        tree_temp_rc.borrow_mut().trees.insert(key,value);
                    }
                }
                if sonpath.clone().is_file()
                {
                    tree_temp_rc.borrow_mut().blobs.insert(sonname.clone(),file_rc.clone());
                }
                else
                {
                    tree_temp_rc.borrow_mut().trees.insert(sonname.clone(),sonobj.clone());

                }
                sonname=fathername.clone();
                sonpath=Path::new(&sonname);
                fathername=fatherName(&(sonname));
                sonobj=tree_temp_rc.clone();
                let SHA1id_temp=getSHA1(&serialize(&sonobj).unwrap());
                objects.treemap.insert(SHA1id_temp.clone(),sonobj.clone());
                reference.borrow_mut().refermap.insert(sonname.clone(),SHA1id_temp.clone());
            }
            if persistence
            {
                let mut f_obj=File::create("./.gitc/maps/objs");
                f_obj.unwrap().write(&serialize(&objects).unwrap());

                let mut f_head=File::create("./.gitc/maps/head");
                f_head.unwrap().write(&serialize(&head).unwrap());
            }
        }
    }
}
```
gitAdd方法接受四个参数，path为被add的文件路径，objects为全局的objects map，head为全局head，persistence为一个布尔类型参数，控制是否进行持久化记录。    
gitAdd方法首先从head中获取当前branch的references map，然后判断path路径对应的是否是个文件，如果path对应一个具体的文件，那将这个文件序列化，计算SHA1，并检查objects和references中是否含有这个文件。如果这个文件没有被包含，则添加这个文件，然后对这个文件所有的父目录进行SHA1计算，并更新或插入objects和references。最后根据persistence来决定是否进行持久化记录head和objects。   
### gitRm
```
pub fn gitRm(path:String,objects:&mut Objects,head:&mut HEAD,persistence:bool)
{
    let mut reference=head.branch.get(&head.currentBranchName).cloned().unwrap().borrow_mut().references.clone();
    let p=Path::new(&(path));
    if p.is_file()
    {
        let filename=path.clone();
        let mut file=load_reference(&reference,objects,filename);
        let SHA1id=getSHA1(&serialize(&file).unwrap());
        if objects.blobmap.contains_key(&SHA1id)
        {
            objects.blobmap.remove(&SHA1id);
        }
        if reference.borrow_mut().refermap.contains_key(&path)
        {
            reference.borrow_mut().refermap.remove(&path);
        }
        let mut fathername=fatherName(&(path));
        let mut sonname=path.clone();
        let mut sonpath=p.clone();
        let mut sonobj=Rc::new(RefCell::new(tree::new(String::from(""))));
        while !fathername.eq("")
        {
            let tree_SHA1=reference.borrow_mut().refermap.get(&fathername).cloned().unwrap();
            let mut tree_rc=objects.treemap.get(&tree_SHA1).cloned().unwrap(); //获取文件父亲树
            let tree_temp=tree_rc.clone();
            if sonpath.clone().is_file()
            {
                tree_temp.borrow_mut().blobs.remove(&sonname.clone());
            }
            sonname=fathername.clone();
            sonpath=Path::new(&sonname);
            fathername=fatherName(&(sonname));
            sonobj=tree_temp.clone();
            let SHA1id_temp=getSHA1(&serialize(&sonobj).unwrap());
            objects.treemap.insert(SHA1id_temp.clone(),sonobj.clone());
            reference.borrow_mut().refermap.insert(sonname.clone(),SHA1id_temp.clone());
        }
        if persistence
        {
            let mut f_obj=File::create("./.gitc/maps/objs");
            f_obj.unwrap().write(&serialize(&objects).unwrap());

            let mut f_head=File::create("./.gitc/maps/head");
            f_head.unwrap().write(&serialize(&head).unwrap());
        }
    }
}
```
gitRm方法接受四个参数，path为被remove的文件路径，objects为全局的objects map，head为全局head，persistence为一个布尔类型参数，控制是否进行持久化记录。    
gitRm方法首先从head中获取当前branch的references map，然后判断path路径对应的是否是个文件，如果path对应一个具体的文件，那将这个文件序列化，计算SHA1，并在objects和references中删除这个文件。然后对这个文件所有的父目录进行SHA1计算，并更新objects和references。最后根据persistence来决定是否进行持久化记录head和objects。   
### gitCommit
```
pub fn gitCommit(head:&mut HEAD,objects:&mut Objects,  message:&str, author:&str,persistence:bool)
{
    let mut reference=head.branch.get(&head.currentBranchName).cloned().unwrap().borrow_mut().references.clone();
    let mut commit=Commit::new();
    commit.message=String::from(message);
    commit.author=String::from(author);

    let snapshot=load_reference_tree(&reference,objects,String::from("."));

    commit.snapshot=snapshot;

    let currentbranch=head.branch.get(&head.currentBranchName.clone()).cloned().unwrap();

    let fathercommit=currentbranch.borrow().commitpointer.clone().unwrap();
    if Some(fathercommit.clone()).is_some()
    {
        for c in 0..fathercommit.borrow().parents.len()
        {
            if let Some(son)=fathercommit.borrow().parents.get(c).cloned()
            {
                commit.parents.push(son);
            }
        }
        if !fathercommit.borrow().message.eq(&String::from(""))
        {
            commit.parents.push(Some(fathercommit));
        }

    }

    let mut commit_rc=Rc::new(RefCell::new(commit));

    let SHA1id_temp=getSHA1(&serialize(&commit_rc).unwrap());
    objects.commitmap.insert(SHA1id_temp.clone(),commit_rc.clone());
    reference.borrow_mut().refermap.insert(String::from(message),SHA1id_temp.clone());

    let mut branchname=head.currentBranchName.clone();

    head.branch.get(&branchname).cloned().unwrap().borrow_mut().commitpointer=Some(commit_rc.clone());
    
    if persistence
    {
        let mut f_obj=File::create("./.gitc/maps/objs");
        f_obj.unwrap().write(&serialize(&objects).unwrap());

        let mut f_head=File::create("./.gitc/maps/head");
        f_head.unwrap().write(&serialize(&head).unwrap());
    }
}
```
gitCommit方法接受五个参数，objects为全局的objects map，head为全局head，message为提交信息，author是提交者的名字，persistence为一个布尔类型参数，控制是否进行持久化记录。   

gitCommit方法首先从head中获取当前branch的references map，然后创建一个新的commit，并利用message和author为新commit赋值，将新commit的所有父commit插入到新commit的parents中，
从references中读出当前的根目录tree，让新commit中的snapshot指向根目录tree。对新commit计算SHA1，插入objects和references，调整当前branch的commit指针位置，让他指向这个新commit，最后根据persistence来决定是否进行持久化记录head和objects。   
### gitBranch
```
pub fn gitBranch(head:&mut HEAD,branchname:String,persistence:bool)
{
    if head.branch.contains_key(&branchname)==false
    {
        let mut newbranch=Branch::new(branchname.clone());

        for (key,value) in head.branch.get(&head.currentBranchName).cloned().unwrap().borrow().references.borrow().refermap.clone()
        {
            newbranch.references.borrow_mut().refermap.insert(key,value);
        }
        newbranch.commitpointer=head.branch.get(&head.currentBranchName).cloned().unwrap().borrow().commitpointer.clone();
        head.branch.insert(branchname.clone(),Rc::new(RefCell::new(newbranch)));
        if persistence
        {
            let mut f_head=File::create("./.gitc/maps/head");
            f_head.unwrap().write(&serialize(&head).unwrap());
        }
    }
    else {
        println!("This name is contained.Try another name");
    }
}
```
gitBranch方法接受三个参数，head为全局head，branchname为新添加的branch的名字，persistence为一个布尔类型参数，控制是否进行持久化记录。    
gitBranch首先判断branchname是否已存在，如果不存在，则用这个branchname创建一个新的branch，并从当前branch中拷贝创建一个新的references作为新branch的references，之后让新branch的commit指针指向当前branch所指的commit上，再将新branch插入到head中。最后根据persistence来决定是否进行持久化记录head。    
### gitCheckout
```
pub fn gitCheckout(head:&mut HEAD,branchname:String,persistence:bool)
{
    if head.branch.contains_key(&branchname)
    {
        head.currentBranchName=branchname;
        if persistence
        {
            let mut f_head=File::create("./.gitc/maps/head");
            f_head.unwrap().write(&serialize(&head).unwrap());
        }
    }
    else {
        println!("uncontained name!");
    }
}
```
gitCheckout方法接受三个参数，head为全局head，branchname为要切换到的branch的名字，persistence为一个布尔类型参数，控制是否进行持久化记录。   
gitCheckout先检查head中是否包含这个名字的branch，如果包含则修改head中的currentBranchName为branchname，如果不包含则提示branchname uncontained。最后根据persistence来决定是否进行持久化记录head。    
### gitMerge
```
pub fn gitMerge(head:&mut HEAD,branch2:String,message:&str,author:&str,objects:&mut Objects,persistence:bool)
{

    let mut reference=head.branch.get(&head.currentBranchName).cloned().unwrap().borrow_mut().references.clone();
    let mut mainbranch=head.branch.get(&head.currentBranchName).cloned().unwrap();
    let mut minorbranch=head.branch.get(&branch2).cloned().unwrap();

    let mut maincommit=mainbranch.borrow().commitpointer.clone().unwrap();
    let mut minorcommit=minorbranch.borrow().commitpointer.clone().unwrap();

    let mut newcommit=Commit::new();
    newcommit.t=objecttype::commit;
    newcommit.message=String::from(message);
    newcommit.author=String::from(author);
    for c in 0..maincommit.borrow().parents.len()
    {
        if let Some(son)=maincommit.borrow().parents.get(c).cloned()
        {
            newcommit.parents.push(son);
        }
    }
    newcommit.parents.push(Some(maincommit.clone()));
    for c in 0..minorcommit.borrow().parents.len()
    {
        if let Some(son)=minorcommit.borrow().parents.get(c).cloned()
        {
            newcommit.mergeparents.push(son);
        }
    }
    newcommit.mergeparents.push(Some(minorcommit.clone()));
    newcommit.snapshot.borrow_mut().name=String::from(".");
    newcommit.snapshot.borrow_mut().t=objecttype::tree;
    newcommit.snapshot.borrow_mut().blobs=maincommit.borrow().snapshot.borrow().blobs.clone();
    newcommit.snapshot.borrow_mut().trees=maincommit.borrow().snapshot.borrow().trees.clone();
    
    let mut whilecount:usize=0;
    let mut mainCurrentTreeNode=newcommit.snapshot.clone();
    let mut minorCurrentTreeNode=minorcommit.borrow_mut().snapshot.clone();
    let mut mainTreeQueue:Vec<Rc<RefCell<tree>>>=vec![];
    mainTreeQueue.push(mainCurrentTreeNode.clone());
    let mut minorTreeQueue:Vec<Rc<RefCell<tree>>>=vec![];
    minorTreeQueue.push(minorCurrentTreeNode.clone());
    while minorTreeQueue.len()!=0
    {
        if whilecount==0
        {
            mainTreeQueue.remove(0);
            minorTreeQueue.remove(0);
        }
        else if whilecount>0
        {
            mainCurrentTreeNode=mainTreeQueue.get(0).cloned().unwrap();
            minorCurrentTreeNode=minorTreeQueue.get(0).cloned().unwrap();
            mainTreeQueue.remove(0);
            minorTreeQueue.remove(0);
        }
        for (key,value) in minorCurrentTreeNode.borrow().blobs.clone()
        {
            //println!("{:?},{:?}",key,value);
            if(mainCurrentTreeNode.borrow().blobs.contains_key(&key))  //也有这个文件 涉及合并问题 之后再搞
            {

            }
            else  //main branch 中没有 直接加入
            {

                mainCurrentTreeNode.borrow_mut().blobs.insert(key.clone(),value.clone());
                let SHA1id_temp=getSHA1(&serialize(&value.clone()).unwrap());
                objects.blobmap.insert(SHA1id_temp.clone(),value.clone());
                reference.borrow_mut().refermap.insert(key.clone(),SHA1id_temp.clone());
            }
        }
        for (key,value) in minorCurrentTreeNode.borrow().trees.clone()
        {
            if(mainCurrentTreeNode.borrow().trees.contains_key(&key))  //都有这个文件夹就进去
            {
                minorTreeQueue.push(value.clone());
                mainTreeQueue.push(mainCurrentTreeNode.borrow().trees.get(&key).cloned().unwrap());
            }
            else
            {
                mainCurrentTreeNode.borrow_mut().trees.insert(key,value);
            }
        }
        whilecount+=1;
    }

    //对所有文件夹的 SHA进行一次更新
    let mut renewcount:usize=0;
    let mut renewCurrentTreeNode=newcommit.snapshot.clone();
    let mut renewTreeQueue:Vec<Rc<RefCell<tree>>>=vec![];
    let mut recordTreeQueue:Vec<Rc<RefCell<tree>>>=vec![];
    let mut recordblobQueue:Vec<Rc<RefCell<blob>>>=vec![];
    renewTreeQueue.push(renewCurrentTreeNode.clone());
    recordTreeQueue.push(renewCurrentTreeNode.clone());

    while renewTreeQueue.len()!=0
    {
        if renewcount==0
        {
            renewTreeQueue.remove(0);
        }
        else if renewcount>0
        {
            renewCurrentTreeNode=renewTreeQueue.get(0).cloned().unwrap();
            renewTreeQueue.remove(0);
        }
        for (key,value) in renewCurrentTreeNode.borrow().blobs.clone()
        {
            recordblobQueue.push(value.clone());
        }
        for (key,value) in renewCurrentTreeNode.borrow().trees.clone()
        {
            recordTreeQueue.push(value.clone());
        }
        renewcount+=1;
    }
    for index in 0..recordblobQueue.len()
    {
        let obj=recordblobQueue.get(index).cloned().unwrap();
        let SHA1id_temp=getSHA1(&serialize(&obj).unwrap());
        objects.blobmap.insert(SHA1id_temp.clone(),obj.clone());
        reference.borrow_mut().refermap.insert(obj.clone().borrow().name.clone(),SHA1id_temp.clone());
    }

    for index in 0..recordTreeQueue.len()
    {
        let obj=recordTreeQueue.get(index).cloned().unwrap();
        let SHA1id_temp=getSHA1(&serialize(&obj).unwrap());

        objects.treemap.insert(SHA1id_temp.clone(),obj.clone());
        reference.borrow_mut().refermap.insert(obj.clone().borrow().name.clone(),SHA1id_temp.clone());
    }


    let mut commit_rc=Rc::new(RefCell::new(newcommit));
    let SHA1id_temp=getSHA1(&serialize(&commit_rc).unwrap());
    objects.commitmap.insert(SHA1id_temp.clone(),commit_rc.clone());
    reference.borrow_mut().refermap.insert(String::from(message),SHA1id_temp.clone());


    head.branch.get(&head.currentBranchName.clone()).cloned().unwrap().borrow_mut().commitpointer=Some(commit_rc.clone());


    if persistence
    {
        let mut f_obj=File::create("./.gitc/maps/objs");
        f_obj.unwrap().write(&serialize(&objects).unwrap());

        let mut f_head=File::create("./.gitc/maps/head");
        f_head.unwrap().write(&serialize(&head).unwrap());
    }
}
```
gitMerge方法接受六个参数，objects为全局的objects map，head为全局head，branch2为被merge的branch的名字，message为提交信息，author是提交者的名字，persistence为一个布尔类型参数，控制是否进行持久化记录。    
首先从head中读出当前分支的references，再读出当前branch所指向的maincommit以及被merge的branch指向的minorcommit，创建一个新commit，并用message和author给它赋值。将maincommit的所有父commit以及他自己插入到新commit的parents中，将minorcommit的所有父commit以及他自己插入到新commit的mergedparents中，利用maincommit对新commit的snapshot进行赋值。把rust中的vec作为类似于队列的数据结构来使用，对新commit的snapshot和minorcommit的snapshot进行广度优先的搜索，minorcommit的snapshot中有，但新commit的snapshot中没有的文件和文件夹，并把这些文件夹和文件加入到新commit的snapshot中。之后采用同样的广度优先搜索方式，再对新commit的snapshot中的tree和blob进行SHA1码的更新。之后计算并插入新commit的SHA1，调整当前branch指向新commit。最后根据persistence来决定是否进行持久化记录head和objects。   
## 5.测试结果
### gitAdd测试
```
#[test]
fn add_test() {
    let mut head:HEAD=HEAD::new();

    head.currentBranchName=String::from("master");
    head.branch.insert(String::from("master"),Rc::new(RefCell::new(Branch::new(String::from("master")))));

    let mut objects=Objects::new();

    gitAdd("./hello.txt".to_string(),&mut objects,&mut head,false);

    gitAdd("./demo/demo1.txt".to_string(),&mut objects,&mut head,false);

    for (k,v) in objects.blobmap
    {
        println!("{:?},{:?}",k,v);
    }
}
```
上面是gitAdd的测试，初始化head和objects，然后gitAdd了两个文件，最后打印objecs的blobmap来查看插入的结果，结果如下。   
```
"8d2a8025cb90d625e0d51c373520e273bae99b5b",RefCell { value: blob { name: "./hello.txt", contents: [104, 101, 108, 108, 111, 48], t: blob } }
"c182014417bd1f1ec0eb5ca8f62dc937ec770c74",RefCell { value: blob { name: "./demo/demo1.txt", contents: [100, 101, 109, 111, 49], t: blob } }
```
可以看到这两条插入的结果。       
### gitRm测试
```
#[test]
fn rm_test() {
    let mut head:HEAD=HEAD::new();

    head.currentBranchName=String::from("master");
    head.branch.insert(String::from("master"),Rc::new(RefCell::new(Branch::new(String::from("master")))));

    let mut objects=Objects::new();

    gitAdd("./hello.txt".to_string(),&mut objects,&mut head,false);

    gitAdd("./demo/demo1.txt".to_string(),&mut objects,&mut head,false);

    gitRm("./hello.txt".to_string(),&mut objects,&mut head,false);

    for (k,v) in objects.blobmap
    {
        println!("{:?},{:?}",k,v);
    }
}
```
在add_test的基础上，利用gitRm删除"./hello.txt"，最后打印objecs的blobmap来查看删除的结果，结果如下。   
```
"c182014417bd1f1ec0eb5ca8f62dc937ec770c74",RefCell { value: blob { name: "./demo/demo1.txt", contents: [100, 101, 109, 111, 49], t: blob } }
```
可以看到"./hello.txt"的记录被删除，只剩"/demo/demo1.txt"的记录。     
### gitCommit测试
```
#[test]
fn commit_test() {
    let mut head:HEAD=HEAD::new();

    head.currentBranchName=String::from("master");
    head.branch.insert(String::from("master"),Rc::new(RefCell::new(Branch::new(String::from("master")))));

    let mut objects=Objects::new();

    gitAdd("./hello.txt".to_string(),&mut objects,&mut head,false);

    gitAdd("./demo/demo1.txt".to_string(),&mut objects,&mut head,false);

    gitCommit(&mut head,&mut objects,"master_first","alex",false);

    for (k,v) in objects.blobmap
    {
        println!("{:?},{:?}",k,v);
    }
    println!("-------------");
    for (k,v) in objects.treemap
    {
        println!("{:?},{:?}",k,v);
    }
    println!("-------------");
    for (k,v) in objects.commitmap
    {
        println!("{:?},{:?}",k,v.borrow().snapshot);
    }
    println!("-------------");
}
```
commit_test在add_test的基础上进行了commit提交，最后打印objecs的三个来查看提交的结果，结果如下。   
blob    
```
"8d2a8025cb90d625e0d51c373520e273bae99b5b",RefCell { value: blob { name: "./hello.txt", contents: [104, 101, 108, 108, 111, 48], t: blob } }
"c182014417bd1f1ec0eb5ca8f62dc937ec770c74",RefCell { value: blob { name: "./demo/demo1.txt", contents: [100, 101, 109, 111, 49], t: blob } }
```
tree    
```
"a4b91d2edfc07bd36b15fc47dae2b7f39113831d",RefCell { value: tree { name: ".", trees: {}, blobs: {"./hello.txt": RefCell { value: blob { name: "./hello.txt", contents: [104, 101, 108, 108, 111, 48], t: blob } }}, t: tree } }
"cb19177bc0eaeebdf0b9f064c745147764ce57e4",RefCell { value: tree { name: "./demo", trees: {}, blobs: {"./demo/demo1.txt": RefCell { value: blob { name: "./demo/demo1.txt", contents: [100, 101, 109, 111, 49], t: blob } }}, t: tree } }
"ceb1fa16da9aa349e7b8da902ddefb2b9c1ca421",RefCell { value: tree { name: ".", trees: {"./demo": RefCell { value: tree { name: "./demo", trees: {}, blobs: {"./demo/demo1.txt": RefCell { value: blob { name: "./demo/demo1.txt", contents: [100, 101, 109, 111, 49], t: blob } }}, t: tree } }}, blobs: {"./hello.txt": RefCell { value: blob { name: "./hello.txt", contents: [104, 101, 108, 108, 111, 48], t: blob } }}, t: tree } }
```
commit    
```
"48bc13ae1caf189bf3ef3adeef1e1c09f73df234",RefCell { value: tree { name: ".", trees: {"./demo": RefCell { value: tree { name: "./demo", trees: {}, blobs: {"./demo/demo1.txt": RefCell { value: blob { name: "./demo/demo1.txt", contents: [100, 101, 109, 111, 49], t: blob } }}, t: tree } }}, blobs: {"./hello.txt": RefCell { value: blob { name: "./hello.txt", contents: [104, 101, 108, 108, 111, 48], t: blob } }}, t: tree } }
```
### gitBranch & gitCheckout测试
```
#[test]
fn branch_test() {
    let mut head:HEAD=HEAD::new();

    head.currentBranchName=String::from("master");
    head.branch.insert(String::from("master"),Rc::new(RefCell::new(Branch::new(String::from("master")))));

    let mut objects=Objects::new();

    gitAdd("./hello.txt".to_string(),&mut objects,&mut head,false);

    gitAdd("./demo/demo1.txt".to_string(),&mut objects,&mut head,false);

    gitCommit(&mut head,&mut objects,"master_first","alex",false);

    gitBranch(&mut head,String::from("b1"),false);

    gitCheckout(&mut head,String::from("b1"),false);

    for (k,v) in head.branch.clone()
    {
        println!("branchname:{:?}",k);
    }
    println!("-------------");
    let branch1=head.branch.get(&String::from("master")).cloned().unwrap();
    let branch2=head.branch.get(&String::from("b1")).cloned().unwrap();

    assert_eq!(branch1.borrow().commitpointer,branch2.borrow().commitpointer);
    assert_eq!(branch1.borrow().references,branch2.borrow().references);

    gitAdd("./hello1.txt".to_string(),&mut objects,&mut head,false);

    gitCommit(&mut head,&mut objects,"b1_first","alex",false);

    println!("{:?}",head.branch.get(&String::from("master")).cloned().unwrap().borrow().references);
    println!("-------------");

    println!("{:?}",head.branch.get(&String::from("b1")).cloned().unwrap().borrow().references);
    println!("-------------");

    for (k,v) in objects.blobmap
    {
        println!("{:?},{:?}",k,v);
    }
    println!("-------------");
    for (k,v) in objects.treemap
    {
        println!("{:?},{:?}",k,v);
    }
    println!("-------------");
    for (k,v) in objects.commitmap
    {
        println!("{:?},{:?}",k,v.borrow().message);
    }
    println!("-------------");
}
```
branch_test同时对gitBranch和gitCheckout两个方法进行了测试。在commit_test的基础上，创建了一个名为"b1"的新分支，并checkout切换到了这个新分支,打印出目前所有的分支。通过assert_eq!来判断两个branch是不是完全一致，之后在新分支上添加"./hello1.txt"，并commit，最后打印出两分支references比较他们间的区别，以及objects的三个map。   
```
Finished test [unoptimized + debuginfo] target(s) in 0.12s
```
测试通过，说明两个branch完全一致
```
branchname:"b1"
branchname:"master"
```
打印出所有分支
```
RefCell { value: References { refermap: {".": "ceb1fa16da9aa349e7b8da902ddefb2b9c1ca421", "./demo": "cb19177bc0eaeebdf0b9f064c745147764ce57e4", "./demo/demo1.txt": "c182014417bd1f1ec0eb5ca8f62dc937ec770c74", "./hello.txt": "8d2a8025cb90d625e0d51c373520e273bae99b5b", "master_first": "48bc13ae1caf189bf3ef3adeef1e1c09f73df234"} } }
RefCell { value: References { refermap: {".": "821795a2b3d2ab23cba8127a9605729c86d96eba", "./demo": "cb19177bc0eaeebdf0b9f064c745147764ce57e4", "./demo/demo1.txt": "c182014417bd1f1ec0eb5ca8f62dc937ec770c74", "./hello.txt": "8d2a8025cb90d625e0d51c373520e273bae99b5b", "./hello1.txt": "9da705f7a30e1354e21b0686f19cdbf5ac72f4d1", "b1_first": "034f9a4b6477f121b16f9b978f3244ebdf9cb31a", "master_first": "48bc13ae1caf189bf3ef3adeef1e1c09f73df234"} } }
```
可看到两组references之间存在差别。      
```
"8d2a8025cb90d625e0d51c373520e273bae99b5b",RefCell { value: blob { name: "./hello.txt", contents: [104, 101, 108, 108, 111, 48], t: blob } }
"9da705f7a30e1354e21b0686f19cdbf5ac72f4d1",RefCell { value: blob { name: "./hello1.txt", contents: [104, 101, 108, 108, 111, 49], t: blob } }
"c182014417bd1f1ec0eb5ca8f62dc937ec770c74",RefCell { value: blob { name: "./demo/demo1.txt", contents: [100, 101, 109, 111, 49], t: blob } }
```
```
"821795a2b3d2ab23cba8127a9605729c86d96eba",RefCell { value: tree { name: ".", trees: {"./demo": RefCell { value: tree { name: "./demo", trees: {}, blobs: {"./demo/demo1.txt": RefCell { value: blob { name: "./demo/demo1.txt", contents: [100, 101, 109, 111, 49], t: blob } }}, t: tree } }}, blobs: {"./hello.txt": RefCell { value: blob { name: "./hello.txt", contents: [104, 101, 108, 108, 111, 48], t: blob } }, "./hello1.txt": RefCell { value: blob { name: "./hello1.txt", contents: [104, 101, 108, 108, 111, 49], t: blob } }}, t: tree } }
"a4b91d2edfc07bd36b15fc47dae2b7f39113831d",RefCell { value: tree { name: ".", trees: {}, blobs: {"./hello.txt": RefCell { value: blob { name: "./hello.txt", contents: [104, 101, 108, 108, 111, 48], t: blob } }}, t: tree } }
"cb19177bc0eaeebdf0b9f064c745147764ce57e4",RefCell { value: tree { name: "./demo", trees: {}, blobs: {"./demo/demo1.txt": RefCell { value: blob { name: "./demo/demo1.txt", contents: [100, 101, 109, 111, 49], t: blob } }}, t: tree } }
"ceb1fa16da9aa349e7b8da902ddefb2b9c1ca421",RefCell { value: tree { name: ".", trees: {"./demo": RefCell { value: tree { name: "./demo", trees: {}, blobs: {"./demo/demo1.txt": RefCell { value: blob { name: "./demo/demo1.txt", contents: [100, 101, 109, 111, 49], t: blob } }}, t: tree } }}, blobs: {"./hello.txt": RefCell { value: blob { name: "./hello.txt", contents: [104, 101, 108, 108, 111, 48], t: blob } }}, t: tree } }
```
```
"034f9a4b6477f121b16f9b978f3244ebdf9cb31a","b1_first"
"48bc13ae1caf189bf3ef3adeef1e1c09f73df234","master_first"
```
objects中3个map。    
### gitMerge测试
```
#[test]
fn merge_test() {
    let mut head:HEAD=HEAD::new();

    head.currentBranchName=String::from("master");
    head.branch.insert(String::from("master"),Rc::new(RefCell::new(Branch::new(String::from("master")))));

    let mut objects=Objects::new();

    gitAdd("./hello.txt".to_string(),&mut objects,&mut head,false);

    gitAdd("./demo/demo1.txt".to_string(),&mut objects,&mut head,false);

    gitCommit(&mut head,&mut objects,"master_first","alex",false);


    gitBranch(&mut head,String::from("b1"),false);

    gitCheckout(&mut head,String::from("b1"),false);

    gitAdd("./hello1.txt".to_string(),&mut objects,&mut head,false);

    gitCommit(&mut head,&mut objects,"b1_first","alex",false);

    gitCheckout(&mut head,String::from("master"),false);

    gitMerge(&mut head,String::from("b1"),"merge_master_b1","alex",&mut objects,false);


    println!("{:?}",head.branch.get(&String::from("master")).cloned().unwrap().borrow().references);
    println!("---------------");
    let commit=head.branch.get(&String::from("master")).cloned().unwrap().clone().borrow().commitpointer.clone();
    println!("{:?}",commit.clone().unwrap().borrow().snapshot.clone());
    println!("---------------");
    for i in commit.clone().unwrap().borrow().parents.clone()
    {
        if i.is_some()
        {
            println!("parentname:{:?}",i.unwrap().borrow().message);
        }
    }
    println!("---------------");
    for i in commit.clone().unwrap().borrow().mergeparents.clone()
    {
        if i.is_some()
        {
            println!("parentname:{:?}",i.unwrap().borrow().message);
        }
    }

}
```
merge_test在branch_test的基础上对master和b1做了merge。合并后打印出master的references，master最新commit的snapshot以及master最新commit的parents和mergedparents来验证文件"./hello1.txt"的添加以及merge后的新commit共有两个分支的commit记录。   
```
RefCell { value: References { refermap: {".": "821795a2b3d2ab23cba8127a9605729c86d96eba", "./demo": "cb19177bc0eaeebdf0b9f064c745147764ce57e4", "./demo/demo1.txt": "c182014417bd1f1ec0eb5ca8f62dc937ec770c74", "./hello.txt": "8d2a8025cb90d625e0d51c373520e273bae99b5b", "./hello1.txt": "9da705f7a30e1354e21b0686f19cdbf5ac72f4d1", "master_first": "48bc13ae1caf189bf3ef3adeef1e1c09f73df234", "merge_master_b1": "2fa0e81b26352b3a57f0167ffa39e36c3afd8113"} } }
```
可以看到master的references中多了"./hello1.txt"这一项。    
```
RefCell { value: tree { name: ".", trees: {"./demo": RefCell { value: tree { name: "./demo", trees: {}, blobs: {"./demo/demo1.txt": RefCell { value: blob { name: "./demo/demo1.txt", contents: [100, 101, 109, 111, 49], t: blob } }}, t: tree } }}, blobs: {"./hello.txt": RefCell { value: blob { name: "./hello.txt", contents: [104, 101, 108, 108, 111, 48], t: blob } }, "./hello1.txt": RefCell { value: blob { name: "./hello1.txt", contents: [104, 101, 108, 108, 111, 49], t: blob } }}, t: tree } }
```
同时master最新commit中的"."目录下包含了在"b1"分支中添加的"./hello1.txt"。     
```
-parents
parentname:"master_first"
---------------
-mergedparents
parentname:"master_first"
parentname:"b1_first"
```
可以看到master最新commit中的parents中含有"master_first"一个父commit，mergedparents中含有"master_first"和"b1_first"两个父commit，这是符合我们测试中git方法的执行的预期结果的。   