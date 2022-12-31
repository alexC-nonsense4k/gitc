

use crate::gitUtils::gitUtils::References;
use crate::gitUtils::gitUtils::HEAD;
use crate::gitUtils::gitUtils::Branch;
use crate::gitUtils::gitUtils::blob;
use crate::gitUtils::gitUtils::Objects;
use crate::gitUtils::gitUtils::tree;
use crate::gitUtils::gitUtils::Commit;
use crate::gitUtils::gitUtils::objecttype;


use serde::{Serialize, Deserialize};
use bincode::{serialize, deserialize};
use std::io::{Bytes, Read, Write};
use std::rc::Rc;
use std::cell::RefCell;
use std::fs;
use std::fs::File;
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::path::Path;
use hex_literal::hex;
use sha1::{Sha1, Digest};
use walkdir::WalkDir;


const Hex:[char;16]=['0','1','2','3','4','5','6','7','8','9','a','b','c','d','e','f'];

pub fn fatherName(path:&str)->String
{
    let p=Path::new(path);
    let mut fileName=path.to_string();
    let namearray=fileName.split("/");
    let arraysize=namearray.clone().count();
    let mut c:usize=1;
    let mut pname=String::new();
    if arraysize==1
    {
        return String::from("");
    }
    else if arraysize==2
    {
        return String::from(".");
    }
    else
    {
        for s in namearray
        {
            if c==arraysize
            {
                break;
            }
            if c>1
            {
                pname.push_str("/");
            }
            pname.push_str(s);
            c=c+1;
        }
        return pname;
    }
}

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
pub fn gitCommit(head:&mut HEAD,objects:&mut Objects,message:&str, author:&str,persistence:bool)
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

pub fn gitInit()
{
    let target_path = Path::new("./.gitc");
    if target_path.exists()==false
    {
        fs::create_dir("./.gitc");

    }
    let head_path=Path::new("./.gitc/HEAD");
    if  head_path.exists()==false
    {
        let mut f=File::create("./.gitc/HEAD");
        f.unwrap().write(b"ref: refs/heads/master");
    }
    let objects_path = Path::new("./.gitc/objects");
    if objects_path.exists()==false
    {
        fs::create_dir("./.gitc/objects");
    }
    let refs_path = Path::new("./.gitc/refs");
    if refs_path.exists()==false
    {
        fs::create_dir("./.gitc/refs");

    }
    let refsh_path = Path::new("./.gitc/refs/heads");
    if refsh_path.exists()==false
    {
        fs::create_dir("./.gitc/refs/heads");

    }
    let refst_path = Path::new("./.gitc/refs/tags");
    if refst_path.exists()==false
    {
        fs::create_dir("./.gitc/refs/tags");

    }

}
pub fn load_reference(references:&Rc<RefCell<References>>,objects:&Objects,name_or_id:String)->Rc<RefCell<blob>>
{
    if references.borrow_mut().refermap.contains_key(&name_or_id)
    {
        let mut res=objects.blobmap.get(references.borrow_mut().refermap.get(&name_or_id).unwrap()).cloned();
        return res.unwrap();
    }
    else
    {
        let mut res=objects.blobmap.get(&name_or_id).cloned();
        return res.unwrap();
    }
}

pub fn load_reference_tree(references:&Rc<RefCell<References>>,objects:&Objects,name_or_id:String)->Rc<RefCell<tree>>
{
    if references.borrow_mut().refermap.contains_key(&name_or_id)
    {
        let mut res=objects.treemap.get(references.borrow_mut().refermap.get(&name_or_id).unwrap()).cloned();
        return res.unwrap();
    }
    else
    {
        let mut res=objects.treemap.get(&name_or_id).cloned();
        return res.unwrap();
    }
}


pub fn getSHA1(data:&[u8])->String
{
    let mut hasher = Sha1::new();
    hasher.update(data);
    let mut res=String::new();
    let result = hasher.finalize();
    for i in result.iter()
    {
        let a1:u8=15;
        let a2:u8=240;
        let r1=i&a1;
        let r2=(i&a2)/16;

        res.push(Hex[r1 as usize]);
        res.push(Hex[r2 as usize]);
    }
    return res;
}